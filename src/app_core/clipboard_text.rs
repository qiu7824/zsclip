//! Plain display text stays independent from the rich clipboard representation.

pub(crate) fn preview(text: &str) -> String {
    if let Some(preview) = super::url_preview::summarize(text) {
        return preview;
    }
    let line = text.replace(['\r', '\n', '\t'], " ");
    let mut chars = line.trim().chars();
    let mut result: String = chars.by_ref().take(72).collect();
    if chars.next().is_some() {
        result.push_str(" ...");
    }
    result
}

pub(crate) fn stored_preview(text: Option<&str>, fallback: &str, rich: bool) -> String {
    if rich {
        if let Some(text) = text.filter(|text| !text.trim().is_empty()) {
            return preview(text);
        }
    }
    text.and_then(super::url_preview::summarize)
        .unwrap_or_else(|| fallback.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn office_display_values_are_not_replaced_by_html_metadata() {
        for text in [
            "E2",
            "00123",
            "1,234.50",
            "2026/11/03",
            "¥9.90",
            "你好 👩‍💻",
            "first\r\nsecond",
            "a\tb",
        ] {
            assert_eq!(
                stored_preview(Some(text), "HTML: wrong", true),
                preview(text)
            );
        }
        assert_eq!(
            stored_preview(Some("HTML is text"), "HTML is text", false),
            "HTML is text"
        );
        assert_eq!(
            stored_preview(None, "ordinary image", false),
            "ordinary image"
        );
        assert_eq!(
            preview(&"中".repeat(73)),
            format!("{} ...", "中".repeat(72))
        );
    }
}
