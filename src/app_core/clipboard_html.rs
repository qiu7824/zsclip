//! Bounded CF_HTML normalization with preserved document context and byte offsets.

pub(crate) const MAX_HTML_BYTES: usize = 2 * 1024 * 1024;
const START: &str = "<!--StartFragment-->";
const END: &str = "<!--EndFragment-->";

#[path = "clipboard_html_safety.rs"]
mod safety;

fn offset(raw: &str, name: &str) -> Option<i64> {
    raw.lines().take(16).find_map(|line| {
        line.strip_prefix(name)?.trim().parse::<i64>().ok()
    })
}

fn has_header(raw: &str) -> bool {
    raw.starts_with("Version:") || raw.starts_with("StartHTML:")
}

fn parts(raw: &str) -> Option<(String, usize, usize)> {
    if raw.is_empty() || raw.len() > MAX_HTML_BYTES || raw.contains('\0') {
        return None;
    }
    if has_header(raw) {
        let start = usize::try_from(offset(raw, "StartFragment:")?).ok()?;
        let end = usize::try_from(offset(raw, "EndFragment:")?).ok()?;
        if start >= end || raw.get(start..end).is_none() {
            return None;
        }
        match (offset(raw, "StartHTML:"), offset(raw, "EndHTML:")) {
            (Some(a), Some(b)) if a >= 0 && b >= 0 => {
                let a = usize::try_from(a).ok()?;
                let b = usize::try_from(b).ok()?;
                if a > start || b < end { return None; }
                let context = raw.get(a..b)?.to_string();
                return Some((context, start - a, end - a));
            }
            (Some(-1), Some(-1)) => return wrap_fragment(raw.get(start..end)?),
            _ => return None,
        }
    }
    let lower = raw.to_ascii_lowercase();
    if let (Some(start), Some(end)) = (lower.find(&START.to_ascii_lowercase()), lower.find(&END.to_ascii_lowercase())) {
        let start = start + START.len();
        if start >= end || raw.get(start..end).is_none() { return None; }
        return Some((raw.to_string(), start, end));
    }
    // A legacy stored value is an HTML fragment without a clipboard header.
    wrap_fragment(raw.trim())
}

fn wrap_fragment(fragment: &str) -> Option<(String, usize, usize)> {
    if fragment.is_empty() { return None; }
    let prefix = format!("<html><body>{START}");
    let start = prefix.len();
    let end = start.checked_add(fragment.len())?;
    Some((format!("{prefix}{fragment}{END}</body></html>"), start, end))
}

pub(crate) fn fragment(raw: &str) -> Option<String> {
    let (context, start, end) = parts(raw)?;
    Some(context.get(start..end)?.to_string())
}

pub(crate) fn normalize(raw: &str) -> Option<String> {
    let (context, start, end) = parts(raw)?;
    let (context, start, end) = safety::minimize(&context, start, end)?;
    let header = |a, b, c, d| format!(
        "Version:1.0\r\nStartHTML:{a:010}\r\nEndHTML:{b:010}\r\nStartFragment:{c:010}\r\nEndFragment:{d:010}\r\n"
    );
    let header_size = header(0usize, 0usize, 0usize, 0usize).len();
    let total = header_size.checked_add(context.len())?;
    if total > MAX_HTML_BYTES { return None; }
    let mut output = header(header_size, total, header_size + start, header_size + end);
    output.push_str(&context);
    Some(output)
}

pub(crate) fn privacy_candidates(normalized: &str) -> Vec<String> {
    let Some((context, _, _)) = parts(normalized) else { return Vec::new(); };
    safety::candidates(&context)
}

pub(crate) fn native_document(raw:&str)->Option<String> {
    let normalized=normalize(raw)?;
    parts(&normalized).map(|(document,_,_)|document)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_styles_and_utf8_fragment_survive_round_trip() {
        let html = "<html><head><style>.xl{color:#ff0000;font-weight:bold}</style></head><body><!--StartFragment--><table><tr><td class=xl>中文😀</td></tr></table><!--EndFragment--></body></html>";
        let encoded = normalize(html).unwrap();
        assert!(encoded.contains("<head><style>.zsclip0"));
        assert_eq!(fragment(&encoded).unwrap(), "<table><tr><td class=\"zsclip0\">中文😀</td></tr></table>");
        assert_eq!(normalize(&encoded).unwrap(), encoded);
        assert_eq!(offset(&encoded, "EndHTML:").unwrap() as usize, encoded.len());
    }

    #[test]
    fn legacy_fragment_is_wrapped_without_inventing_styles() {
        let encoded = normalize("<b>颜色</b>").unwrap();
        assert_eq!(fragment(&encoded).as_deref(), Some("<b>颜色</b>"));
        assert!(!encoded.contains("<style>"));
    }

    #[test]
    fn invalid_offsets_nuls_and_large_payloads_are_rejected() {
        assert!(normalize("Version:1.0\r\nStartHTML:10\r\nEndHTML:999\r\nStartFragment:20\r\nEndFragment:22\r\n").is_none());
        assert!(normalize("<p>bad\0text</p>").is_none());
        assert!(normalize(&"a".repeat(MAX_HTML_BYTES + 1)).is_none());
        let encoded = normalize("<p>中</p>").unwrap();
        let start = offset(&encoded, "StartFragment:").unwrap() as usize;
        let bad = encoded.replace(&format!("StartFragment:{start:010}"), &format!("StartFragment:{:010}", start + 4));
        assert!(normalize(&bad).is_none());
    }

    #[test]
    fn clipboard_context_contains_only_selected_text_and_inert_styles() {
        let raw = "<html><head><meta content='outside-secret'><style>.outside-secret{color:#ff0000;font-weight:bold;font-size:14pt;background-color:#ffff00;border:1px solid #000000;content:'outside-secret'}</style><!--outside-secret--></head><body><p>ordinary outside-secret context</p><div data-private='outside-secret'>before outside-secret<!--StartFragment--><table><tr><td class='outside-secret' title='outside-secret' data-private='outside-secret'>中文😀<!--outside-secret--></td></tr></table><!--EndFragment-->after outside-secret</div></body></html>";
        let html = normalize(raw).unwrap();
        assert!(!html.contains("outside-secret"));
        assert!(html.contains(".zsclip0{color:#ff0000;font-weight:bold;font-size:14pt;background-color:#ffff00;border:1px solid #000000}"));
        assert!(fragment(&html).unwrap().contains("class=\"zsclip0\">中文😀"));
        assert_eq!(normalize(&html).unwrap(), html);
        assert_eq!(offset(&html, "EndHTML:").unwrap() as usize, html.len());
    }

    #[test]
    fn hidden_or_executable_fragment_falls_back_to_plain_text() {
        for fragment in ["<span hidden>secret</span>", "<span style='display:none'>secret</span>", "<script>secret</script>", "<span class=unresolved>secret</span>"] {
            assert!(normalize(fragment).is_none(), "{fragment}");
        }
    }
}
