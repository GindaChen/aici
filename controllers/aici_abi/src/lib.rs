pub use toktrie;
pub use toktrie::{bytes, recognizer, rng};
pub use toktrie::{SimpleVob, TokenizerEnv};

use serde::{Deserialize, Serialize};

mod host;

#[cfg(feature = "cfg")]
pub mod cfg;
#[cfg(feature = "cfg")]
mod lex;

#[cfg(feature = "rx")]
pub mod rx;

pub mod dlex;

pub mod substring;

pub type TokenId = toktrie::TokenId;

pub use host::{
    aici_stop, arg_bytes, arg_string, get_config, host_trie, self_seq_id, tokenize, tokenize_bytes,
    StorageCmd, StorageOp, StorageResp, VariableStorage, WasmTokenizerEnv,
};

#[cfg(not(target_arch = "wasm32"))]
pub use host::{set_host, HostInterface};

#[derive(Serialize, Deserialize, Debug)]
pub struct InitPromptArg {
    pub prompt: Vec<TokenId>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitPromptResult {
    pub prompt: Vec<TokenId>,
}

impl InitPromptResult {
    pub fn from_arg(arg: InitPromptArg) -> Self {
        InitPromptResult { prompt: arg.prompt }
    }
}

#[repr(transparent)]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct SeqId(pub u32);

#[derive(Serialize, Deserialize, Debug)]
pub struct MidProcessArg {
    /// Sampling result for the previous iteration.
    /// For simple sampled token 't', backtrack==0 and tokens==[t].
    /// For first request, backtrack==0 and tokens==[] (prompt is passed separately, before).
    /// Can be more complex when splices are used.
    pub backtrack: u32,
    pub tokens: Vec<TokenId>,
    /// The token that was sampled, before splicing, if any.
    pub sampled: Option<TokenId>,
    ///
    pub fork_group: Vec<SeqId>,
}

impl MidProcessArg {
    pub fn has_eos(&self) -> bool {
        let eos = host::eos_token();
        self.tokens.iter().any(|t| *t == eos)
    }

    pub fn save_tokens(&self, acc_tokens: &mut Vec<TokenId>) {
        let bt = self.backtrack as usize;
        assert!(
            bt <= acc_tokens.len(),
            "attempting to backtrack past beginning"
        );
        acc_tokens.truncate(acc_tokens.len() - bt);
        acc_tokens.extend_from_slice(&self.tokens);
    }
}

pub use toktrie::{Branch, Splice};

#[derive(Debug)]
pub struct MidProcessResult {
    /// Fork the request into multiple branches.
    /// Typically, exactly one branch is returned.
    /// If multiple branches are returned, they are executed in parallel.
    /// If no branches are returned, the request is terminated.
    pub branches: Vec<Branch<SimpleVob>>,
}

impl MidProcessResult {
    pub fn from_branch(branch: Branch<SimpleVob>) -> Self {
        if branch.is_stop() {
            Self::stop()
        } else {
            MidProcessResult {
                branches: vec![branch],
            }
        }
    }

    pub fn stop() -> Self {
        MidProcessResult { branches: vec![] }
    }

    pub fn sample(set: SimpleVob) -> Self {
        Self::sample_with_temp(set, None)
    }

    pub fn sample_with_temp(set: SimpleVob, temperature: Option<f32>) -> Self {
        Self::from_branch(Branch::sample(set, temperature))
    }

    pub fn splice(backtrack: u32, ff_tokens: Vec<TokenId>) -> Self {
        Self::from_branch(Branch::splice(backtrack, ff_tokens))
    }

    pub fn noop() -> Self {
        Self::splice(0, vec![])
    }

    pub fn is_stop(&self) -> bool {
        self.branches.is_empty()
    }
}

#[derive(Serialize, Deserialize)]
pub struct ProcessResultOffset {
    /// Branches use byte offsets into the bias tensor.
    pub branches: Vec<Branch<usize>>,
}

pub trait AiciCtrl {
    /// Called with the initial prompt. ~1000ms time limit.
    /// By default ignore prompt.
    fn init_prompt(&mut self, arg: InitPromptArg) -> InitPromptResult {
        InitPromptResult::from_arg(arg)
    }

    /// This is the main entry point for the module. ~20ms time limit.
    fn mid_process(&mut self, arg: MidProcessArg) -> MidProcessResult;

    // Internals
    fn aici_init_prompt(&mut self) {
        let arg: InitPromptArg = serde_json::from_slice(&host::process_arg_bytes()).unwrap();
        let res = self.init_prompt(arg);
        let res_bytes = serde_json::to_vec(&res).unwrap();
        host::return_process_result(&res_bytes);
    }

    fn aici_mid_process(&mut self) {
        let arg: MidProcessArg = serde_json::from_slice(&host::process_arg_bytes())
            .expect("aici_mid_process: failed to deserialize MidProcessArg");
        let res = self.mid_process(arg);
        let mut used_logits = false;
        let res = ProcessResultOffset {
            branches: res
                .branches
                .into_iter()
                .map(|b| {
                    b.map_mask(|vob| {
                        if used_logits {
                            panic!("aici_mid_process: multiple branches with sampling not yet supported");
                        }
                        used_logits = true;
                        host::return_logit_bias(&vob) as usize
                    })
                })
                .collect(),
        };
        let res_bytes = serde_json::to_vec(&res).expect("aici_mid_process: failed to serialize");
        host::return_process_result(&res_bytes);
    }
}

/// Expose method as extern "C", usage:
///     expose!(Foo::set_count(n: i32) -> i32);
/// Generates "C" function:
///     set_count(Foo *, i32) -> i32
#[macro_export]
macro_rules! expose {
    ($struct_name:ident :: $method_name:ident ( $($arg:ident : $typ:ty),* ) -> $ret:ty) => {
        #[no_mangle]
        pub extern "C" fn $method_name(self_: *mut $struct_name, $($arg : $typ),*) -> $ret {
            unsafe {
                (&mut *self_).$method_name($($arg),*)
            }
        }
    };
    ($struct_name:ident :: $field:ident :: $method_name:ident ( $($arg:ident : $typ:ty),* ) -> $ret:ty) => {
        #[no_mangle]
        pub extern "C" fn $method_name(self_: *mut $struct_name, $($arg : $typ),*) -> $ret {
            unsafe {
                (&mut *self_).$field.$method_name($($arg),*)
            }
        }
    };
}

#[macro_export]
macro_rules! aici_expose_all {
    ($struct_name:ident, $new:expr) => {
        $crate::expose!($struct_name::aici_mid_process() -> ());
        $crate::expose!($struct_name::aici_init_prompt() -> ());

        #[no_mangle]
        pub extern "C" fn aici_create() -> *mut $struct_name {
            let b = Box::new($new);
            Box::into_raw(b)
        }

        #[no_mangle]
        pub extern "C" fn aici_panic() {
            panic!("aici_panic()")
        }
    }
}

#[macro_export]
macro_rules! include_bytes_aligned {
    ($align_ty:ty, $path:literal) => {{
        #[repr(C)] // guarantee 'bytes' comes after '_align'
        pub struct AlignedAs<Align, Bytes: ?Sized> {
            pub _align: [Align; 0],
            pub bytes: Bytes,
        }

        // this assignment is made possible by CoerceUnsized
        static ALIGNED: &AlignedAs<$align_ty, [u8]> = &AlignedAs {
            _align: [],
            bytes: *include_bytes!($path),
        };

        &ALIGNED.bytes
    }};
}
