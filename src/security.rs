use regex::Regex;

pub fn is_suspicious(content: &str) -> bool {
    let patterns = [
        r#"(?i)(api_key|apikey|secret|token).{0,20}['|"][0-9a-zA-Z]{32,45}['|"]"#,
        r"ghp_[0-9a-zA-Z]{36}",
        r"sk_live_[0-9a-zA-Z]{24}",
    ];

    for p in patterns {
        if let Ok(re) = Regex::new(p) {
            if re.is_match(content) {
                return true;
            }
        }
    }
    false
}
