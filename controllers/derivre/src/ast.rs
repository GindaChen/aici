use crate::{hashcons::VecHashMap, pp::PrettyPrinter};
use bytemuck_derive::{Pod, Zeroable};

#[derive(Pod, Zeroable, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ExprRef(u32);

impl ExprRef {
    pub const INVALID: ExprRef = ExprRef(0);
    pub const EMPTY_STRING: ExprRef = ExprRef(1);
    pub const NO_MATCH: ExprRef = ExprRef(2);
    pub const ANY_BYTE: ExprRef = ExprRef(3);
    pub const ANY_STRING: ExprRef = ExprRef(4);
    pub const NON_EMPTY_STRING: ExprRef = ExprRef(5);

    pub fn new(id: u32) -> Self {
        assert!(id != 0, "ExprRef(0) is reserved for invalid reference");
        ExprRef(id)
    }

    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }
    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

pub enum Expr<'a> {
    EmptyString,
    NoMatch,
    Byte(u8),
    ByteSet(&'a [u32]),
    Lookahead(ExprFlags, ExprRef, u32),
    Not(ExprFlags, ExprRef),
    Repeat(ExprFlags, ExprRef, u32, u32),
    Concat(ExprFlags, &'a [ExprRef]),
    Or(ExprFlags, &'a [ExprRef]),
    And(ExprFlags, &'a [ExprRef]),
}

#[derive(Clone, Copy)]
pub struct ExprFlags(u32);
impl ExprFlags {
    const NULLABLE: ExprFlags = ExprFlags(1 << 8);
    const ZERO: ExprFlags = ExprFlags(0);

    pub fn is_nullable(&self) -> bool {
        self.0 & ExprFlags::NULLABLE.0 != 0
    }

    fn from_nullable(nullable: bool) -> Self {
        if nullable {
            Self::NULLABLE
        } else {
            Self::ZERO
        }
    }

    fn encode(&self, tag: ExprTag) -> u32 {
        self.0 | tag as u32
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum ExprTag {
    EmptyString = 1,
    NoMatch,
    Byte,
    ByteSet,
    Lookahead,
    Not,
    Repeat,
    Concat,
    Or,
    And, // has to be last, see below
}

impl ExprTag {
    const MAX_VAL: u8 = ExprTag::And as u8;
    fn from_u8(x: u8) -> Self {
        if x == 0 || x > Self::MAX_VAL {
            panic!("invalid tag: {x}");
        }
        unsafe { std::mem::transmute(x) }
    }
}

#[inline(always)]
pub fn byteset_contains(s: &[u32], b: usize) -> bool {
    s[b / 32] & (1 << (b % 32)) != 0
}

#[inline(always)]
pub fn byteset_set(s: &mut [u32], b: usize) {
    s[b / 32] |= 1 << (b % 32);
}

#[inline(always)]
pub fn byteset_union(s: &mut [u32], other: &[u32]) {
    for i in 0..s.len() {
        s[i] |= other[i];
    }
}

pub fn byteset_256() -> Vec<u32> {
    vec![0u32; 256 / 32]
}

pub fn byteset_from_range(start: u8, end: u8) -> Vec<u32> {
    assert!(start <= end, "start: {start}, end: {end}");
    let mut s = byteset_256();
    for b in start..=end {
        byteset_set(&mut s, b as usize);
    }
    s
}

impl<'a> Expr<'a> {
    pub fn matches_byte(&self, b: u8) -> bool {
        match self {
            Expr::EmptyString => false,
            Expr::NoMatch => false,
            Expr::Byte(b2) => b == *b2,
            Expr::ByteSet(s) => byteset_contains(s, b as usize),
            _ => panic!("not a simple expression"),
        }
    }

    fn get_flags(&self) -> ExprFlags {
        match self {
            Expr::EmptyString => ExprFlags::NULLABLE,
            Expr::NoMatch | Expr::Byte(_) | Expr::ByteSet(_) => ExprFlags::ZERO,
            Expr::Lookahead(f, _, _) => *f,
            Expr::Not(f, _) => *f,
            Expr::Repeat(f, _, _, _) => *f,
            Expr::Concat(f, _) => *f,
            Expr::Or(f, _) => *f,
            Expr::And(f, _) => *f,
        }
    }

    pub fn nullable(&self) -> bool {
        self.get_flags().is_nullable()
    }

    fn from_slice(s: &'a [u32]) -> Expr<'a> {
        let flags = ExprFlags(s[0] & !0xff);
        let tag = ExprTag::from_u8((s[0] & 0xff) as u8);
        match tag {
            ExprTag::EmptyString => Expr::EmptyString,
            ExprTag::NoMatch => Expr::NoMatch,
            ExprTag::Byte => Expr::Byte(s[1] as u8),
            ExprTag::ByteSet => Expr::ByteSet(&s[1..]),
            ExprTag::Lookahead => Expr::Lookahead(flags, ExprRef::new(s[1]), s[2]),
            ExprTag::Not => Expr::Not(flags, ExprRef::new(s[1])),
            ExprTag::Repeat => Expr::Repeat(flags, ExprRef::new(s[1]), s[2], s[3]),
            ExprTag::Concat => Expr::Concat(flags, bytemuck::cast_slice(&s[1..])),
            ExprTag::Or => Expr::Or(flags, bytemuck::cast_slice(&s[1..])),
            ExprTag::And => Expr::And(flags, bytemuck::cast_slice(&s[1..])),
        }
    }

    fn serialize(&self, trg: &mut VecHashMap) {
        #[inline(always)]
        fn nary_serialize(trg: &mut VecHashMap, tag: u32, es: &[ExprRef]) {
            trg.insert_u32(tag);
            trg.insert_slice(bytemuck::cast_slice(es));
        }
        let zf = ExprFlags::ZERO;
        match self {
            Expr::EmptyString => trg.insert_u32(zf.encode(ExprTag::EmptyString)),
            Expr::NoMatch => trg.insert_u32(zf.encode(ExprTag::NoMatch)),
            Expr::Byte(b) => {
                trg.insert_slice(&[zf.encode(ExprTag::Byte), *b as u32]);
            }
            Expr::ByteSet(s) => {
                trg.insert_u32(zf.encode(ExprTag::ByteSet));
                trg.insert_slice(s);
            }
            Expr::Lookahead(flags, e, n) => {
                trg.insert_slice(&[flags.encode(ExprTag::Lookahead), e.0, *n]);
            }
            Expr::Not(flags, e) => trg.insert_slice(&[flags.encode(ExprTag::Not), e.0]),
            Expr::Repeat(flags, e, a, b) => {
                trg.insert_slice(&[flags.encode(ExprTag::Repeat), e.0, *a, *b])
            }
            Expr::Concat(flags, es) => nary_serialize(trg, flags.encode(ExprTag::Concat), es),
            Expr::Or(flags, es) => nary_serialize(trg, flags.encode(ExprTag::Or), es),
            Expr::And(flags, es) => nary_serialize(trg, flags.encode(ExprTag::And), es),
        }
    }
}

pub struct ExprSet {
    exprs: VecHashMap,
    alphabet_size: usize,
    alphabet_words: usize,
    pp: PrettyPrinter,
}

impl ExprSet {
    pub fn new(alphabet_size: usize) -> Self {
        let exprs = VecHashMap::new();
        let alphabet_words = (alphabet_size + 31) / 32;
        let mut r = ExprSet {
            exprs,
            alphabet_size,
            alphabet_words,
            pp: PrettyPrinter::new_simple(alphabet_size),
        };

        let inserts = vec![
            (r.mk(Expr::EmptyString), ExprRef::EMPTY_STRING),
            (r.mk(Expr::NoMatch), ExprRef::NO_MATCH),
            (
                r.mk(Expr::ByteSet(&vec![0xffffffff; alphabet_words])),
                ExprRef::ANY_BYTE,
            ),
            (
                r.mk(Expr::Repeat(
                    ExprFlags::NULLABLE,
                    ExprRef::ANY_BYTE,
                    0,
                    u32::MAX,
                )),
                ExprRef::ANY_STRING,
            ),
            (
                r.mk(Expr::Repeat(
                    ExprFlags::ZERO,
                    ExprRef::ANY_BYTE,
                    1,
                    u32::MAX,
                )),
                ExprRef::NON_EMPTY_STRING,
            ),
        ];

        for (x, y) in inserts {
            assert!(x == y, "id: {x:?}, expected: {y:?}");
        }

        r
    }

    pub fn set_pp(&mut self, pp: PrettyPrinter) {
        self.pp = pp;
    }

    pub fn pp(&self) -> &PrettyPrinter {
        &self.pp
    }

    pub fn expr_to_string(&self, id: ExprRef) -> String {
        self.pp.expr_to_string(&self, id)
    }

    pub fn expr_to_string_with_info(&self, id: ExprRef) -> String {
        let mut r = self.expr_to_string(id);
        r.push_str(&self.pp.alphabet_info());
        r
    }

    pub fn alphabet_size(&self) -> usize {
        self.alphabet_size
    }

    pub fn alphabet_words(&self) -> usize {
        self.alphabet_words
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.exprs.len()
    }

    pub fn bytes(&self) -> usize {
        self.exprs.num_bytes()
    }

    pub fn mk_byte(&mut self, b: u8) -> ExprRef {
        self.mk(Expr::Byte(b))
    }

    pub fn mk_byte_set(&mut self, s: &[u32]) -> ExprRef {
        assert!(s.len() == self.alphabet_words);
        let mut num_set = 0;
        for x in s.iter() {
            num_set += x.count_ones();
        }
        if num_set == 0 {
            ExprRef::NO_MATCH
        } else if num_set == 1 {
            for i in 0..self.alphabet_size {
                if byteset_contains(s, i) {
                    return self.mk_byte(i as u8);
                }
            }
            unreachable!()
        } else {
            self.mk(Expr::ByteSet(s))
        }
    }

    pub fn mk_repeat(&mut self, e: ExprRef, min: u32, max: u32) -> ExprRef {
        if e == ExprRef::NO_MATCH {
            if min == 0 {
                ExprRef::EMPTY_STRING
            } else {
                ExprRef::NO_MATCH
            }
        } else if min > max {
            panic!();
            // ExprRef::NO_MATCH
        } else if max == 0 {
            ExprRef::EMPTY_STRING
        } else if min == 1 && max == 1 {
            e
        } else {
            let min = if self.is_nullable(e) { 0 } else { min };
            let flags = ExprFlags::from_nullable(min == 0);
            self.mk(Expr::Repeat(flags, e, min, max))
        }
    }

    // pub fn mk_star(&mut self, e: ExprRef) -> ExprRef {
    //     self.mk_repeat(e, 0, u32::MAX)
    // }

    // pub fn mk_plus(&mut self, e: ExprRef) -> ExprRef {
    //     self.mk_repeat(e, 1, u32::MAX)
    // }

    fn flatten_tag(&self, exp_tag: ExprTag, args: Vec<ExprRef>) -> Vec<ExprRef> {
        let mut i = 0;
        while i < args.len() {
            let tag = self.get_tag(args[i]);
            if tag == exp_tag {
                // ok, we found tag, we can no longer return the original vector
                let mut res = args[0..i].to_vec();
                while i < args.len() {
                    let tag = self.get_tag(args[i]);
                    if tag != exp_tag {
                        res.push(args[i]);
                    } else {
                        res.extend_from_slice(self.get_args(args[i]));
                    }
                    i += 1;
                }
                return res;
            }
            i += 1;
        }
        args
    }

    pub fn mk_or(&mut self, mut args: Vec<ExprRef>) -> ExprRef {
        // TODO deal with byte ranges
        args = self.flatten_tag(ExprTag::Or, args);
        args.sort_by_key(|&e| e.0);
        let mut dp = 0;
        let mut prev = ExprRef::NO_MATCH;
        let mut nullable = false;
        let mut num_bytes = 0;
        let mut num_lookahead = 0;
        for idx in 0..args.len() {
            let arg = args[idx];
            if arg == prev || arg == ExprRef::NO_MATCH {
                continue;
            }
            if arg == ExprRef::ANY_STRING {
                return ExprRef::ANY_STRING;
            }
            match self.get(arg) {
                Expr::Byte(_) | Expr::ByteSet(_) => {
                    num_bytes += 1;
                }
                Expr::Lookahead(_, _, _) => {
                    num_lookahead += 1;
                }
                _ => {}
            }
            if !nullable && self.is_nullable(arg) {
                nullable = true;
            }
            args[dp] = arg;
            dp += 1;
            prev = arg;
        }
        args.truncate(dp);

        // TODO we should probably do sth similar in And
        if num_bytes > 1 {
            let mut byteset = vec![0u32; self.alphabet_words];
            args.retain(|&e| {
                let n = self.get(e);
                match n {
                    Expr::Byte(b) => {
                        byteset_set(&mut byteset, b as usize);
                        false
                    }
                    Expr::ByteSet(s) => {
                        byteset_union(&mut byteset, s);
                        false
                    }
                    _ => true,
                }
            });
            let node = self.mk_byte_set(&byteset);
            add_to_sorted(&mut args, node);
        }

        if num_lookahead > 1 {
            let mut lookahead = vec![];
            args.retain(|&e| {
                let n = self.get(e);
                match n {
                    Expr::Lookahead(_, inner, n) => {
                        lookahead.push((e, inner, n));
                        false
                    }
                    _ => true,
                }
            });
            lookahead.sort_by_key(|&(_, e, n)| (e.0, n));

            let mut prev = ExprRef::INVALID;
            for idx in 0..lookahead.len() {
                let (l, inner, _) = lookahead[idx];
                if inner == prev {
                    continue;
                }
                prev = inner;
                args.push(l);
            }

            args.sort_by_key(|&e| e.0);
        }

        if args.len() == 0 {
            ExprRef::NO_MATCH
        } else if args.len() == 1 {
            args[0]
        } else {
            let flags = ExprFlags::from_nullable(nullable);
            self.mk(Expr::Or(flags, &args))
        }
    }

    pub fn mk_and(&mut self, mut args: Vec<ExprRef>) -> ExprRef {
        args = self.flatten_tag(ExprTag::And, args);
        args.sort_by_key(|&e| e.0);
        let mut dp = 0;
        let mut prev = ExprRef::ANY_STRING;
        let mut had_empty = false;
        let mut nullable = true;
        for idx in 0..args.len() {
            let arg = args[idx];
            if arg == prev || arg == ExprRef::ANY_STRING {
                continue;
            }
            if arg == ExprRef::NO_MATCH {
                return ExprRef::NO_MATCH;
            }
            if arg == ExprRef::EMPTY_STRING {
                had_empty = true;
            }
            if nullable && !self.is_nullable(arg) {
                nullable = false;
            }
            args[dp] = arg;
            dp += 1;
            prev = arg;
        }
        args.truncate(dp);

        if args.len() == 0 {
            ExprRef::ANY_STRING
        } else if args.len() == 1 {
            args[0]
        } else if had_empty {
            if nullable {
                ExprRef::EMPTY_STRING
            } else {
                ExprRef::NO_MATCH
            }
        } else {
            let flags = ExprFlags::from_nullable(nullable);
            self.mk(Expr::And(flags, &args))
        }
    }

    pub fn mk_concat(&mut self, mut args: Vec<ExprRef>) -> ExprRef {
        args = self.flatten_tag(ExprTag::Concat, args);
        args.retain(|&e| e != ExprRef::EMPTY_STRING);
        if args.len() == 0 {
            ExprRef::EMPTY_STRING
        } else if args.len() == 1 {
            args[0]
        } else if args.iter().any(|&e| e == ExprRef::NO_MATCH) {
            ExprRef::NO_MATCH
        } else {
            let flags = ExprFlags::from_nullable(args.iter().all(|&e| self.is_nullable(e)));
            self.mk(Expr::Concat(flags, &args))
        }
    }

    pub fn mk_not(&mut self, e: ExprRef) -> ExprRef {
        if e == ExprRef::EMPTY_STRING {
            ExprRef::NON_EMPTY_STRING
        } else if e == ExprRef::NON_EMPTY_STRING {
            ExprRef::EMPTY_STRING
        } else if e == ExprRef::ANY_STRING {
            ExprRef::NO_MATCH
        } else if e == ExprRef::NO_MATCH {
            ExprRef::ANY_STRING
        } else {
            let n = self.get(e);
            match n {
                Expr::Not(_, e2) => return e2,
                _ => {}
            }
            let flags = ExprFlags::from_nullable(!n.nullable());
            self.mk(Expr::Not(flags, e))
        }
    }

    pub fn mk_lookahead(&mut self, mut e: ExprRef, offset: u32) -> ExprRef {
        if e == ExprRef::NO_MATCH {
            return ExprRef::NO_MATCH;
        }

        let flags = if self.is_nullable(e) {
            e = ExprRef::EMPTY_STRING;
            ExprFlags::NULLABLE
        } else {
            ExprFlags::ZERO
        };
        self.mk(Expr::Lookahead(flags, e, offset))
    }

    fn mk(&mut self, e: Expr) -> ExprRef {
        self.exprs.start_insert();
        e.serialize(&mut self.exprs);
        ExprRef(self.exprs.finish_insert())
    }

    pub fn get(&self, id: ExprRef) -> Expr {
        Expr::from_slice(self.exprs.get(id.0))
    }

    fn lookahead_len_inner(&self, e: ExprRef) -> Option<usize> {
        match self.get(e) {
            Expr::Lookahead(_, ExprRef::EMPTY_STRING, n) => Some(n as usize),
            _ => None,
        }
    }

    pub fn lookahead_len(&self, e: ExprRef) -> Option<usize> {
        match self.get(e) {
            Expr::Or(_, args) => args
                .iter()
                .filter_map(|&arg| self.lookahead_len_inner(arg))
                .min(),
            _ => self.lookahead_len_inner(e),
        }
    }

    fn get_flags(&self, id: ExprRef) -> ExprFlags {
        assert!(id.is_valid());
        if id == ExprRef::EMPTY_STRING {
            return ExprFlags::NULLABLE;
        }
        ExprFlags(self.exprs.get(id.0)[0] & !0xff)
    }

    fn get_tag(&self, id: ExprRef) -> ExprTag {
        assert!(id.is_valid());
        let tag = self.exprs.get(id.0)[0] & 0xff;
        ExprTag::from_u8(tag as u8)
    }

    pub fn get_args(&self, id: ExprRef) -> &[ExprRef] {
        let s = self.exprs.get(id.0);
        let tag = ExprTag::from_u8((s[0] & 0xff) as u8);
        match tag {
            ExprTag::Concat | ExprTag::Or | ExprTag::And => bytemuck::cast_slice(&s[1..]),
            ExprTag::Not | ExprTag::Repeat | ExprTag::Lookahead => bytemuck::cast_slice(&s[1..2]),
            ExprTag::EmptyString | ExprTag::NoMatch | ExprTag::Byte | ExprTag::ByteSet => &[],
        }
    }

    pub fn is_nullable(&self, id: ExprRef) -> bool {
        self.get_flags(id).is_nullable()
    }
}

fn add_to_sorted(args: &mut Vec<ExprRef>, e: ExprRef) {
    let idx = args.binary_search(&e).unwrap_or_else(|x| x);
    assert!(idx == args.len() || args[idx] != e);
    args.insert(idx, e);
}
