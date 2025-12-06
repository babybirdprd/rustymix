use tiktoken_rs::cl100k_base;

pub fn count_tokens(content: &str) -> usize {
    let bpe = cl100k_base().unwrap();
    bpe.encode_with_special_tokens(content).len()
}

pub fn is_binary(content: &[u8]) -> bool {
    let len = std::cmp::min(content.len(), 8192);
    content[0..len].contains(&0)
}
