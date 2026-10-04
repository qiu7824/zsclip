//! Minimal inert text/table HTML. Non-selected document data is never retained.
use std::collections::HashMap;

fn tag_end(input: &str, start: usize) -> Option<usize> {
    let mut quote = None;
    for (offset, ch) in input[start..].char_indices() {
        match (quote, ch) {
            (Some(q), c) if q == c => quote = None,
            (None, '\'' | '"') => quote = Some(ch),
            (None, '>') => return Some(start + offset + 1),
            _ => {}
        }
    }
    None
}

fn allowed_tag(name: &str) -> bool {
    matches!(name, "table" | "thead" | "tbody" | "tfoot" | "tr" | "td" | "th" | "colgroup" | "col"
        | "span" | "p" | "div" | "b" | "strong" | "i" | "em" | "u" | "s" | "strike"
        | "del" | "sup" | "sub" | "br" | "pre" | "code" | "ol" | "ul" | "li" | "font"
        | "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

fn dangerous_tag(name: &str) -> bool {
    matches!(name, "script" | "style" | "template" | "noscript" | "iframe" | "object" | "embed" | "input"
        | "textarea" | "select" | "option" | "svg" | "math")
}

fn escape(value: &str) -> String {
    value.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")
}

pub(super) fn decode(value: &str) -> String {
    let mut out = String::new();
    let mut rest = value;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find(';').filter(|n| *n <= 16) else {
            out.push('&'); rest = &rest[start + 1..]; continue;
        };
        let entity = &rest[start + 1..start + end];
        let decoded = match entity {
            "amp" => Some('&'), "lt" => Some('<'), "gt" => Some('>'), "quot" => Some('"'),
            "apos" | "#39" => Some('\''), "nbsp" => Some(' '),
            _ if entity.starts_with("#x") || entity.starts_with("#X") => u32::from_str_radix(&entity[2..],16).ok().and_then(char::from_u32),
            _ if entity.starts_with('#') => entity[1..].parse::<u32>().ok().and_then(char::from_u32),
            _ => None,
        };
        if let Some(ch) = decoded { out.push(ch); } else { out.push_str(&rest[start..=start + end]); }
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    out
}

fn without_comments(value: &str) -> Option<String> {
    let mut out = String::new(); let mut rest = value;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        let end = rest[start + 2..].find("*/")? + start + 4;
        rest = &rest[end..];
    }
    out.push_str(rest); Some(out)
}

fn clean_style(value: &str) -> Option<String> {
    let value = without_comments(&decode(value))?;
    let mut declarations = Vec::new();
    for declaration in value.split(';') {
        let Some((property, value)) = declaration.split_once(':') else { continue; };
        let property = property.trim().to_ascii_lowercase();
        let value = value.trim();
        let lower = value.to_ascii_lowercase();
        // Dropping a hiding declaration could expose text not present in the
        // clipboard's plain representation. Fall back to plain text instead.
        if matches!(property.as_str(), "display" | "visibility" | "mso-hide" | "opacity")
            && !matches!(lower.as_str(), "inline" | "block" | "visible" | "1") { return None; }
        let allowed = matches!(property.as_str(), "color" | "background" | "background-color" | "font-family" | "font-size"
            | "font-weight" | "font-style" | "text-decoration" | "text-align" | "vertical-align"
            | "white-space" | "border" | "border-top" | "border-right" | "border-bottom" | "border-left"
            | "border-width" | "border-style" | "border-color" | "border-collapse" | "border-spacing"
            | "padding" | "padding-top" | "padding-right" | "padding-bottom" | "padding-left"
            | "margin" | "margin-top" | "margin-right" | "margin-bottom" | "margin-left"
            | "width" | "height");
        if !allowed { continue; }
        if lower.contains("url") || lower.contains("expression") || lower.contains("var(")
            || value.contains(['<', '>', '\\', '@', '{', '}']) { continue; }
        if property == "font-size" && (lower.starts_with("0;") || lower == "0" || lower.starts_with("0px") || lower.starts_with("0pt")) { return None; }
        if property == "color" && (lower.contains("transparent") || lower.starts_with("rgba")) { return None; }
        if property == "font-family" {
            let known = ["calibri","arial","times new roman","cambria","consolas","courier new","tahoma","verdana",
                "segoe ui","segoe ui variable text","arial unicode ms","宋体","微软雅黑","等线","黑体","仿宋","楷体",
                "sans-serif","serif","monospace","system-ui"];
            if !value.split(',').all(|name| known.contains(&name.trim().trim_matches(['\'', '"']).to_lowercase().as_str())) { continue; }
        } else {
            let keywords = ["px","pt","em","rem","ex","in","cm","mm","pc","rgb","hsl","rgba","hsla","important",
                "solid","dashed","dotted","double","none","thin","medium","thick","bold","bolder","normal","italic","oblique",
                "underline","line-through","left","right","center","justify","top","middle","bottom","baseline","nowrap","pre","pre-wrap",
                "collapse","separate","auto","inherit","initial","unset","lighter","small","large","transparent",
                "black","white","red","green","blue","yellow","gray","grey","silver","maroon","purple","fuchsia","lime","olive","navy","teal","aqua","orange","windowtext","window"];
            let mut safe = true;
            let mut words = String::new(); let mut in_hex = false;
            for ch in lower.chars() {
                if ch == '#' { in_hex = true; words.push(' '); }
                else if in_hex && ch.is_ascii_hexdigit() { continue; }
                else { in_hex = false; words.push(ch); }
            }
            for word in words.split(|c: char| !c.is_ascii_alphabetic() && c != '-') {
                if !word.is_empty() && !keywords.contains(&word) { safe = false; break; }
            }
            if lower.starts_with('#') && !lower[1..].bytes().all(|c| c.is_ascii_hexdigit()) { safe = false; }
            if !safe { continue; }
        }
        declarations.push(format!("{property}:{value}"));
    }
    Some(declarations.join(";"))
}

fn attrs(mut raw: &str) -> Option<Vec<(String, String)>> {
    let mut values = Vec::new();
    while !raw.trim_start().trim_end_matches('/').trim().is_empty() {
        raw = raw.trim_start();
        let end = raw.find(|c: char| c.is_whitespace() || c == '=' || c == '/').unwrap_or(raw.len());
        if end == 0 { break; }
        let name = raw[..end].to_ascii_lowercase(); raw = raw[end..].trim_start();
        let mut value = String::new();
        if let Some(next) = raw.strip_prefix('=') {
            raw = next.trim_start();
            if raw.starts_with(['\'', '"']) {
                let quote = raw.as_bytes()[0] as char; raw = &raw[1..];
                let end = raw.find(quote)?; value = decode(&raw[..end]); raw = &raw[end + 1..];
            } else {
                let end = raw.find(char::is_whitespace).unwrap_or(raw.len());
                value = decode(raw[..end].trim_end_matches('/')); raw = &raw[end..];
            }
        }
        values.push((name, value));
    }
    Some(values)
}

fn clean_tag(raw: &str, classes: &HashMap<String,String>) -> Option<(String, String, bool, bool)> {
    let raw = raw.trim_start_matches('<').trim_end_matches('>').trim();
    let closing = raw.starts_with('/');
    let raw = raw.trim_start_matches('/');
    let end = raw.find(|c: char| c.is_whitespace() || c == '/').unwrap_or(raw.len());
    let name = raw[..end].to_ascii_lowercase();
    if dangerous_tag(&name) { return None; }
    if !allowed_tag(&name) { return Some((name, String::new(), closing, false)); }
    let void = matches!(name.as_str(), "br" | "col");
    if closing { return Some((name.clone(), format!("</{name}>"), true, void)); }
    let mut result = format!("<{name}");
    for (attribute, value) in attrs(&raw[end..])? {
        if attribute == "hidden" || attribute == "aria-hidden" && value == "true" { return None; }
        let cleaned = match attribute.as_str() {
            "style" => clean_style(&value)?,
            "class" => {
                let mut mapped = Vec::new();
                for class in value.split_whitespace() {
                    if let Some(name) = classes.get(class) { mapped.push(name.clone()); }
                    else if !matches!(class, "MsoNormal" | "MsoNormalTable") { return None; }
                }
                mapped.join(" ")
            },
            "colspan" | "rowspan" | "width" | "height" | "border" | "cellspacing" | "cellpadding" | "size"
                if value.bytes().all(|c| c.is_ascii_digit() || matches!(c, b'%' | b'.' | b'+' | b'-')) => value,
            "align" | "valign" if matches!(value.to_ascii_lowercase().as_str(), "left"|"right"|"center"|"justify"|"top"|"middle"|"bottom"|"baseline") => value,
            "bgcolor" | "color" => { let css = clean_style(&format!("color:{value}"))?; if css.is_empty() { continue; } value },
            "face" => { let css = clean_style(&format!("font-family:{value}"))?; if css.is_empty() { continue; } value },
            _ => continue,
        };
        if !cleaned.is_empty() { result.push_str(&format!(" {attribute}=\"{}\"", escape(&cleaned))); }
    }
    result.push('>'); Some((name, result, false, void))
}

fn selected(raw: &str, classes: &HashMap<String,String>) -> Option<String> {
    let mut out = String::new(); let mut cursor = 0;
    while let Some(relative) = raw[cursor..].find('<') {
        let start = cursor + relative; out.push_str(&raw[cursor..start]);
        if raw[start..].starts_with("<!--") {
            cursor = start + raw[start + 4..].find("-->")? + 7; continue;
        }
        let end = tag_end(raw, start)?;
        let (_, tag, _, _) = clean_tag(&raw[start..end], classes)?;
        out.push_str(&tag); cursor = end;
    }
    out.push_str(&raw[cursor..]); Some(out)
}

pub(super) fn minimize(context: &str, start: usize, end: usize) -> Option<(String, usize, usize)> {
    let lower = context.to_ascii_lowercase();
    let mut classes = HashMap::new();
    let mut styles = String::new(); let mut pos = 0;
    while let Some(index) = lower[pos..].find("<style") {
        let begin = pos + index; let open_end = tag_end(context, begin)?;
        let close = lower[open_end..].find("</style")? + open_end;
        let css = without_comments(&context[open_end..close])?;
        let compact = css.to_ascii_lowercase().split_whitespace().collect::<String>();
        if compact.contains("display:none") || compact.contains("visibility:hidden") || compact.contains("mso-hide:all") { return None; }
        let mut rest = css.as_str();
        while let Some(brace) = rest.find('{') {
            let Some(close) = rest[brace + 1..].find('}') else { break; };
            let close = close + brace + 1;
            let selector = rest[..brace].trim();
            let declarations = clean_style(&rest[brace + 1..close])?;
            if !declarations.is_empty() {
                let mut candidate_classes = classes.clone();
                if let Some(selector) = rewrite_selector(selector, &mut candidate_classes) {
                    classes = candidate_classes;
                    styles.push_str(&format!("{selector}{{{declarations}}}"));
                }
            }
            rest = &rest[close + 1..];
        }
        pos = tag_end(context, close)?;
    }
    // Preserve only structural ancestors, never their text or private metadata.
    let prefix = &context[..start]; let mut ancestors: Vec<(String,String)> = Vec::new(); let mut pos = 0;
    while let Some(index) = prefix[pos..].find('<') {
        let begin = pos + index;
        if prefix[begin..].starts_with("<!--") {
            pos = begin + prefix[begin + 4..].find("-->")? + 7; continue;
        }
        let Some(end) = tag_end(prefix, begin) else { break; };
        let raw = &prefix[begin..end];
        let cleaned = clean_tag(raw, &classes);
        if cleaned.is_none() {
            let name = raw.trim_start_matches('<').trim_start_matches('/').split(|c:char| c.is_whitespace() || c == '>').next().unwrap_or("").to_ascii_lowercase();
            if allowed_tag(&name) { return None; }
        }
        if let Some((name, tag, closing, void)) = cleaned {
            if closing {
                if let Some(index) = ancestors.iter().rposition(|(entry,_)| entry == &name) { ancestors.truncate(index); }
            } else if !tag.is_empty() && !void { ancestors.push((name,tag)); }
        }
        pos = end;
    }
    let fragment = selected(context.get(start..end)?, &classes)?;
    let mut document = String::from("<html><head>");
    if !styles.is_empty() { document.push_str(&format!("<style>{styles}</style>")); }
    document.push_str("</head><body>");
    for (_, tag) in &ancestors { document.push_str(tag); }
    document.push_str("<!--StartFragment-->");
    let start = document.len(); document.push_str(&fragment); let end = document.len();
    document.push_str("<!--EndFragment-->");
    for (name,_) in ancestors.iter().rev() { document.push_str(&format!("</{name}>")); }
    document.push_str("</body></html>");
    Some((document,start,end))
}

fn rewrite_selector(selector: &str, classes: &mut HashMap<String,String>) -> Option<String> {
    let mut rest = selector; let mut out = String::new();
    while !rest.is_empty() {
        let c = rest.chars().next()?;
        if c == '.' {
            rest = &rest[1..];
            let end = rest.find(|c:char| !c.is_ascii_alphanumeric() && !matches!(c,'_'|'-')).unwrap_or(rest.len());
            if end == 0 { return None; }
            let next = format!("zsclip{}", classes.len());
            let name = classes.entry(rest[..end].to_string()).or_insert(next);
            out.push('.'); out.push_str(name); rest = &rest[end..];
        } else if c.is_ascii_alphabetic() {
            let end = rest.find(|c:char| !c.is_ascii_alphanumeric()).unwrap_or(rest.len());
            let name = rest[..end].to_ascii_lowercase();
            if !allowed_tag(&name) && !matches!(name.as_str(),"html"|"body") { return None; }
            out.push_str(&name); rest = &rest[end..];
        } else if c.is_ascii_whitespace() || matches!(c,','|'>'|'+'|'*') {
            out.push(c); rest = &rest[c.len_utf8()..];
        } else { return None; }
    }
    (!out.is_empty()).then_some(out)
}

/// Candidates in retained text and formatting values. Private attributes and
/// comments have already been removed by minimize; no outside text is retained.
pub(super) fn candidates(raw: &str) -> Vec<String> {
    let mut values = Vec::new(); let mut cursor = 0;
    while let Some(index) = raw[cursor..].find('<') {
        let start = cursor + index;
        let text = decode(raw[cursor..start].trim());
        if !text.is_empty() { values.push(text); }
        let Some(end) = tag_end(raw,start) else { break; };
        let tag = raw[start + 1..end - 1].trim();
        let offset = tag.find(char::is_whitespace).unwrap_or(tag.len());
        if let Some(attributes) = attrs(&tag[offset..]) {
            for (_, value) in attributes { if !value.is_empty() { values.push(value); } }
        }
        cursor = end;
    }
    let text = decode(raw[cursor..].trim()); if !text.is_empty() { values.push(text); }
    values
}
