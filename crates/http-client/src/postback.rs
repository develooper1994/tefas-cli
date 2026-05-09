//! ASP.NET WebForms postback helpers and HTML form utilities.

/// Extract every `<input type="hidden">` field from raw HTML.
///
/// Parses the HTML linearly (no DOM construction) to collect `(name, value)` pairs,
/// preserving document order.  Handles double-quoted, single-quoted, and unquoted
/// attribute values and is case-insensitive for tag/attribute names.
///
/// Useful for harvesting ASP.NET WebForms state fields (`__VIEWSTATE`,
/// `__VIEWSTATEGENERATOR`, `__EVENTVALIDATION`) before a postback POST.
pub fn extract_hidden_fields(html: &str) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    let lower = html.to_ascii_lowercase();
    let mut pos = 0;

    while let Some(rel) = lower[pos..].find("<input") {
        let start = pos + rel;
        // Find end of this tag (the first `>` after the opening).
        let tag_end = lower[start..]
            .find('>')
            .map_or(html.len(), |i| start + i + 1);
        let tag = &html[start..tag_end];
        let tag_lower = &lower[start..tag_end];
        pos = tag_end;

        if !is_hidden_input(tag_lower) {
            continue;
        }

        if let Some(name) = extract_attr(tag, tag_lower, "name") {
            let value = extract_attr(tag, tag_lower, "value").unwrap_or_default();
            fields.push((name, value));
        }
    }
    fields
}

fn is_hidden_input(tag_lower: &str) -> bool {
    tag_lower.contains("type=\"hidden\"")
        || tag_lower.contains("type='hidden'")
        // unquoted: type=hidden followed by whitespace, >, or /
        || tag_lower
            .find("type=hidden")
            .is_some_and(|p| {
                let after = &tag_lower[p + "type=hidden".len()..];
                after
                    .chars()
                    .next()
                    .is_none_or(|c| c.is_ascii_whitespace() || c == '>' || c == '/')
            })
}

fn extract_attr(tag: &str, tag_lower: &str, attr: &str) -> Option<String> {
    let search = format!("{attr}=");
    let pos = tag_lower.find(&search)?;
    let rest = &tag[pos + search.len()..];
    Some(parse_attr_value(rest))
}

fn parse_attr_value(s: &str) -> String {
    let s = s.trim_start();
    if let Some(rest) = s.strip_prefix('"') {
        rest.split('"').next().unwrap_or("").to_string()
    } else if let Some(rest) = s.strip_prefix('\'') {
        rest.split('\'').next().unwrap_or("").to_string()
    } else {
        // Unquoted: read until whitespace or `>`.
        s.split(|c: char| c.is_ascii_whitespace() || c == '>')
            .next()
            .unwrap_or("")
            .to_string()
    }
}

/// Encode a list of `(name, value)` pairs as `application/x-www-form-urlencoded`.
///
/// Follows the HTML specification:
/// - Letters, digits, `*`, `-`, `.`, `_` are kept as-is.
/// - Spaces are encoded as `+`.
/// - All other bytes are percent-encoded as `%XX`.
pub fn form_urlencode(fields: &[(String, String)]) -> Vec<u8> {
    let mut out = String::new();
    for (i, (key, val)) in fields.iter().enumerate() {
        if i > 0 {
            out.push('&');
        }
        url_encode_into(&mut out, key);
        out.push('=');
        url_encode_into(&mut out, val);
    }
    out.into_bytes()
}

fn url_encode_into(out: &mut String, s: &str) {
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'*' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                let hi = b >> 4;
                let lo = b & 0x0f;
                out.push(
                    char::from_digit(u32::from(hi), 16)
                        .unwrap()
                        .to_ascii_uppercase(),
                );
                out.push(
                    char::from_digit(u32::from(lo), 16)
                        .unwrap()
                        .to_ascii_uppercase(),
                );
            }
        }
    }
}
