use serde::{Deserialize, Serialize};
use svob::SimpleVob;

pub mod bytes;
mod host;
pub mod recognizer;
pub mod rng;
pub mod svob;
pub mod toktree;

pub type TokenId = bytes::TokenId;

pub use host::{
    _print, arg_bytes, return_logit_bias, self_seq_id, stdout, tokenize, tokenize_bytes,
    StorageCmd, StorageOp, StorageResp, VariableStorage,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct InitPromptArg {
    pub prompt: Vec<TokenId>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct InitPromptResult {}

#[repr(transparent)]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct SeqId(pub u32);

#[derive(Serialize, Deserialize, Debug)]
pub struct PreProcessArg {}

#[derive(Serialize, Deserialize, Debug)]
pub struct PreProcessResult {
    /// If no attention masks are returned - stop the sequence.
    /// If one is returned - just continue with this mask.
    /// If more than one attention mask is returned - fork the generation.
    /// Attention mask of length 0 is equivalent [1.0, ..., 1.0].
    /// Otherwise, length of the mask should be the same as the number of prompt + generated tokens.
    pub attention_masks: Vec<Vec<f32>>,

    pub suspend: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MidProcessArg {
    /// fork_group.len() == attention_masks.len().
    /// Use host::self_seq_id() to get the ID of the current sequence.
    pub fork_group: Vec<SeqId>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MidProcessResult {
    /// Stop the current sequence.
    /// Similar to strong bias to EOS.
    Stop,

    /// Sample next token in the current sequence
    SampleWithBias {
        #[serde(skip)]
        allowed_tokens: SimpleVob,
    },

    /// First pop `backtrack` tokens,
    /// then force next tokens to be generated to be `ff_tokens`.
    /// `backtrack` count includes the token about to be generated from this step.
    /// `backtrack` can be 0, and `ff_tokens` can be empty but not both.
    Splice {
        backtrack: u32,
        ff_tokens: Vec<TokenId>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PostProcessArg {
    /// Generally, issued after each token generated by the model.
    /// `tokens` is typically just this one token, except for the
    /// cases when fast-forward tokens are used.
    pub tokens: Vec<TokenId>,

    /// Typically 0.
    pub backtrack: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PostProcessResult {
    /// If true, stop the sequence.
    pub stop: bool,
}

impl PostProcessResult {
    pub fn stop() -> Self {
        PostProcessResult { stop: true }
    }

    pub fn continue_() -> Self {
        PostProcessResult { stop: false }
    }

    pub fn from_arg(arg: &PostProcessArg) -> Self {
        let stop = arg.tokens.contains(&host::eos_token());
        PostProcessResult { stop }
    }
}

impl PreProcessResult {
    pub fn new(attention_masks: Vec<Vec<f32>>) -> Self {
        PreProcessResult {
            attention_masks,
            suspend: false,
        }
    }
    pub fn continue_() -> Self {
        PreProcessResult::new(vec![vec![]])
    }
    pub fn suspend() -> Self {
        PreProcessResult {
            attention_masks: vec![vec![]],
            suspend: true,
        }
    }
    pub fn stop() -> Self {
        PreProcessResult::new(vec![])
    }
}

pub trait AiciVm {
    /// Called with the initial prompt. ~1000ms time limit.
    /// By default ignore prompt.
    fn init_prompt(&mut self, _arg: InitPromptArg) -> InitPromptResult {
        InitPromptResult::default()
    }

    /// Called before mid_process(), can return attention masks. ~1ms time limit.
    /// Should be stateless.
    fn pre_process(&mut self, _arg: PreProcessArg) -> PreProcessResult {
        PreProcessResult::continue_()
    }

    /// This is the main entry point for the module. ~20ms time limit.
    fn mid_process(&mut self, arg: MidProcessArg) -> MidProcessResult;

    /// Called after tokens are appended, after mid_process(). ~1ms time limit.
    fn post_process(&mut self, arg: PostProcessArg) -> PostProcessResult {
        PostProcessResult::from_arg(&arg)
    }

    // Internals
    fn aici_init_prompt(&mut self) {
        let arg: InitPromptArg = serde_json::from_slice(&host::process_arg_bytes()).unwrap();
        let res = self.init_prompt(arg);
        let res_bytes = serde_json::to_vec(&res).unwrap();
        host::return_process_result(&res_bytes);
    }

    fn aici_pre_process(&mut self) {
        let arg: PreProcessArg = serde_json::from_slice(&host::process_arg_bytes()).unwrap();
        let res = self.pre_process(arg);
        let res_bytes = serde_json::to_vec(&res).unwrap();
        host::return_process_result(&res_bytes);
    }

    fn aici_mid_process(&mut self) {
        let arg: MidProcessArg = serde_json::from_slice(&host::process_arg_bytes()).unwrap();
        let res = self.mid_process(arg);
        match &res {
            MidProcessResult::SampleWithBias { allowed_tokens } => {
                if allowed_tokens.len() > 0 {
                    host::return_logit_bias(allowed_tokens);
                }
            }
            _ => {}
        }
        let res_bytes = serde_json::to_vec(&res).unwrap();
        host::return_process_result(&res_bytes);
    }

    fn aici_post_process(&mut self) {
        let arg: PostProcessArg = serde_json::from_slice(&host::process_arg_bytes()).unwrap();
        let res = self.post_process(arg);
        let res_bytes = serde_json::to_vec(&res).unwrap();
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
        $crate::expose!($struct_name::aici_pre_process() -> ());
        $crate::expose!($struct_name::aici_mid_process() -> ());
        $crate::expose!($struct_name::aici_post_process() -> ());
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

#[macro_export]
macro_rules! wprintln {
    () => {
        $crate::_print("\n")
    };
    ($($arg:tt)*) => {{
        $crate::_print(&format!($($arg)*));
        $crate::_print("\n");
    }};
}

#[macro_export]
macro_rules! wprint {
    ($($arg:tt)*) => {{
        $crate::_print(&format!($($arg)*));
    }};
}
