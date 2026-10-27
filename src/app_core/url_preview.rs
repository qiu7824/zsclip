//! Bounded original URL text for resizable clipboard lists.
pub(crate) const MAX_URL_BYTES: usize = 8192;

pub(crate) fn summarize(text: &str) -> Option<String> {
    if text.len() > MAX_URL_BYTES {
        return None;
    }
    let text = text.trim();
    let (scheme, rest) = text.split_once("://")?;
    if !(scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http"))
        || rest.is_empty()
        || text.chars().any(|ch| ch.is_whitespace() || ch.is_control())
    {
        return None;
    }
    // Rendering applies the actual row width; paste loads the full original.
    Some(text.chars().take(1024).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn url_preview_keeps_original_link_beyond_the_old_72_character_limit() {
        let url = format!(
            "https://docs.qq.com/{}/DOCUMENT_A?tab=000001",
            "shared-prefix/".repeat(8)
        );
        assert_eq!(summarize(&url), Some(url.clone()));
        assert_ne!(
            summarize(&url),
            summarize(&url.replace("DOCUMENT_A", "DOCUMENT_B"))
        );
    }
    #[test]
    fn previews_are_bounded_and_other_text_uses_normal_text_preview() {
        assert_eq!(
            summarize(&format!("https://example.com/{}", "文".repeat(1200)))
                .unwrap()
                .chars()
                .count(),
            1024
        );
        assert!(summarize(&"x".repeat(8193)).is_none());
        assert!(summarize("ordinary text").is_none());
        assert!(summarize("https://one.example/ https://two.example/").is_none());
    }
}
