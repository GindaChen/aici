use gvm_abi::{
    recognizer::{compute_bias, Uppercase},
    toktree::TokTrie, printing::init_panic, wprintln,
};

fn main() {
    init_panic();

    let trie = TokTrie::from_env();
    for idx in 1000..1001 {
        let bytes = trie.token(idx);
        wprintln!("{}: {:?}", idx, String::from_utf8_lossy(bytes));
    }

    let mut logits = vec![0.0; trie.vocab_size() + 1];
    let rec = Uppercase::new();
    for _ in 0..1000 {
        compute_bias(&trie, rec, &mut logits);
    }

    wprintln!("res: {}", logits.iter().filter(|x| **x > -50.0).count());
}