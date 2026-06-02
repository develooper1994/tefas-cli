mod types;
mod validation;
pub use types::{FundPageMeta, FundPageOutput};

pub(crate) use validation::contains_failureconfig;

/// Parse a TEFAS fund page document and return a typed [`FundPageOutput`].
///
/// This is an ergonomic wrapper around [`parse_document`] that replaces the unnamed
/// `(Value, Value)` tuple with a documented struct.
pub fn parse_document_typed(content: &str) -> FundPageOutput {
    let (data, meta_fields) = parse_document(content);
    FundPageOutput::from_raw(data, meta_fields)
}

// A simple helper to extract the page title (example usage in tests)
pub(crate) fn extract_title(html: &str) -> Option<String> {
    TITLE_RE
        .captures(html)
        .and_then(|m| m.get(1))
        .map(|m| decode_html_entities(&clean_whitespace(Some(m.as_str()))))
        .filter(|s| !s.is_empty())
}

fn prune_grouped_output(mut grouped: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;

    let Some(root) = grouped.as_object_mut() else {
        return grouped;
    };

    root.remove("history");

    if let Some(Value::Object(indicator)) = root.get_mut("indicator") {
        for key in ["gunluk_getiri_raw", "pay_adet_raw", "son_fiyat_tl_raw"] {
            indicator.shift_remove(key);
        }
    }

    if let Some(Value::Object(profile)) = root.get_mut("profile") {
        for key in [
            "fon_kodu_raw",
            "fon_toplam_deger_tl_raw",
            "isin_kodu_raw",
            "kap_bilgi_adresi_raw",
            "kategori_raw",
            "max_alis_islem_miktari_raw",
            "max_satis_islem_miktari_raw",
            "min_alis_islem_miktari_raw",
            "min_satis_islem_miktari_raw",
            "yatirimci_sayisi_raw",
        ] {
            profile.shift_remove(key);
        }
    }

    if let Some(Value::Object(ret)) = root.get_mut("return") {
        for key in [
            "son_1ay_getiri_raw",
            "son_1yil_getiri_raw",
            "son_3ay_getiri_raw",
            "son_3yil_getiri_raw",
            "son_5yil_getiri_raw",
            "son_6ay_getiri_raw",
        ] {
            ret.shift_remove(key);
        }
    }

    grouped
}

fn normalize_parse_output(
    grouped: serde_json::Value,
    meta: serde_json::Value,
) -> (serde_json::Value, serde_json::Value) {
    (prune_grouped_output(grouped), meta)
}

// Public helper: parse and normalize grouped/meta outputs.
pub fn parse_document(content: &str) -> (serde_json::Value, serde_json::Value) {
    if content.trim().is_empty() {
        return (
            serde_json::json!({"indicator": {}, "profile": {}, "return": {}}),
            serde_json::json!({}),
        );
    }
    let (grouped, meta) = parse_html_text(content);
    normalize_parse_output(grouped, meta)
}

use aho_corasick::AhoCorasick;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{Map, Value, json};

const FALLBACK_LABELS: &[&str] = &[
    "Son Fiyat (TL)",
    "Günlük Getiri (%)",
    "Pay (Adet)",
    "Son 1 Ay Getirisi",
    "Son 3 Ay Getirisi",
    "Son 6 Ay Getirisi",
    "Son 1 Yıl Getirisi",
    "Son 3 Yıl Getirisi",
    "Son 5 Yıl Getirisi",
];

const RETURN_SPECS: &[(&str, &str, &str)] = &[
    ("Son 1 Ay Getirisi", "son_1ay_getiri_raw", "son_1ay_getiri_pct"),
    ("Son 3 Ay Getirisi", "son_3ay_getiri_raw", "son_3ay_getiri_pct"),
    ("Son 6 Ay Getirisi", "son_6ay_getiri_raw", "son_6ay_getiri_pct"),
    ("Son 1 Yıl Getirisi", "son_1yil_getiri_raw", "son_1yil_getiri_pct"),
    ("Son 3 Yıl Getirisi", "son_3yil_getiri_raw", "son_3yil_getiri_pct"),
    ("Son 5 Yıl Getirisi", "son_5yil_getiri_raw", "son_5yil_getiri_pct"),
];

// Canonical profile label -> output key mapping reused across parse calls.
static PROFILE_MAPPING: &[(&str, &str)] = &[
    ("Fon Kodu", "fon_kodu"),
    ("Kodu", "fon_kodu"),
    ("ISIN Kodu", "isin_kodu"),
    ("Platform Durumu", "platform_islem_durumu"),
    ("Platform İşlem Durumu", "platform_islem_durumu"),
    ("İşlem Başlama Saati", "islem_baslangic_saati"),
    ("Son İşlem Saati", "son_islem_saati"),
    ("Fon Alış Valörü", "fon_alis_valoru"),
    ("Fon Satış Valörü", "fon_satis_valoru"),
    ("Min. Alış İşlem Miktarı", "min_alis_islem_miktari"),
    ("Min. Satış İşlem Miktarı", "min_satis_islem_miktari"),
    ("Max. Alış İşlem Miktarı", "max_alis_islem_miktari"),
    ("Max. Satış İşlem Miktarı", "max_satis_islem_miktari"),
    ("Giriş Komisyonu", "giris_komisyonu"),
    ("Çıkış Komisyonu", "cikis_komisyonu"),
    ("Fonun Faiz İçeriği", "fon_faiz_icerigi"),
    ("Fonun Risk Değeri", "fon_risk_degeri"),
    ("Fon Risk Değeri", "fon_risk_degeri"),
    ("KAP Bilgi Adresi", "kap_bilgi_adresi"),
    ("Fon Toplam Değer (TL)", "fon_toplam_deger_tl"),
    ("Fon Toplam Değer", "fon_toplam_deger_tl"),
    ("Kategorisi", "kategori"),
    ("Kategori", "kategori"),
    (
        "Son 1 Yıllık Kategori Derecesi",
        "son_1yillik_kategori_derecesi",
    ),
    ("Yatırımcı Sayısı", "yatirimci_sayisi"),
    ("Yatirimci Sayisi", "yatirimci_sayisi"),
    ("Pazar Payı", "pazar_payi_raw_pct"),
];

static PROFILE_MAPPING_NORMALIZED: Lazy<Vec<(&'static str, &'static str, String)>> =
    Lazy::new(|| {
        PROFILE_MAPPING
            .iter()
            .map(|(label, key)| (*label, *key, normalize_label(label)))
            .collect()
    });

static FALLBACK_LABEL_MATCHER: Lazy<AhoCorasick> =
    Lazy::new(|| AhoCorasick::new(FALLBACK_LABELS).unwrap());

static THOUSANDS_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d{1,3}(?:\.\d{3})+$").unwrap());
static TRAILING_COMMA_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#",\s*([\]\}])"#).unwrap());
static TITLE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?is)<title[^>]*>(.*?)</title>"#).unwrap());
static SERIES_KEY_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?i)series"#).unwrap());
static XAXIS_KEY_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?i)xaxis"#).unwrap());

// Statics for patterns used inside parse_html_text, extract_allocation_from_table, etc.
// Compiled once on first use instead of per call.
static H3_VARLIK_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?is)<h3[^>]*>\s*Fon Varlık Dağılımı\s*</h3>").unwrap());
static TABLE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)<table[^>]*>(.*?)</table>").unwrap());
static TR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)<tr[^>]*>(.*?)</tr>").unwrap());
static CELL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)<t[dh][^>]*>(.*?)</t[dh]>").unwrap());
static FUND_NAME_ID_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?is)<span[^>]*id=['"]MainContent_FormViewMainIndicators_LabelFund['"][^>]*>(.*?)</span>"#).unwrap()
});
static H6_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?is)<h6[^>]*>([^<]+)</h6>"#).unwrap());
static MAIN_INDICATORS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?is)<div[^>]*class=['"]main-indicators['"][^>]*>(.*?)</div>"#).unwrap()
});
static LI_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?is)<li[^>]*>(.*?)</li>"#).unwrap());
static SPAN_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?is)<span[^>]*>(.*?)</span>"#).unwrap());
static LI_LABEL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?is)^(.*?)<br"#).unwrap());
static P_PAIR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?is)<p[^>]*>([^<]+)</p>\s*<p[^>]*>([^<]*)</p>"#).unwrap());
static INFO_SCOPE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?is)<h3[^>]*>\s*Fon Bilgisi\s*</h3>"#).unwrap());
static DETAILS_VIEW_TABLE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?is)<table[^>]*id\s*=\s*['"]?MainContent_DetailsViewFund['"]?[^>]*>(.*?)</table>"#,
    )
    .unwrap()
});
static ANCHOR_HREF_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?is)<a[^>]*href=['"]([^'"]+)['"]"#).unwrap());
static SERIES_INIT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?is)series\s*:\s*\["#).unwrap());

static KAP_LINK_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?is)\\?"kapLink\\?"\s*:\s*\\?"([^"\\]+)"#).unwrap());

fn clean_whitespace(s: Option<&str>) -> String {
    match s {
        Some(t) => {
            let without_comments = if t.contains("<!--") {
                strip_html_comments(t)
            } else {
                std::borrow::Cow::Borrowed(t)
            };

            let trimmed = without_comments.trim();
            if trimmed.is_empty() {
                return String::new();
            }
            if trimmed.len() == without_comments.len()
                && !trimmed.contains("  ")
                && !trimmed
                    .as_bytes()
                    .iter()
                    .any(|b| matches!(b, b'\n' | b'\r' | b'\t'))
            {
                return trimmed.to_string();
            }

            if trimmed.is_ascii() {
                return collapse_ascii_whitespace(trimmed);
            }

            let mut words = trimmed.split_whitespace();
            let Some(first) = words.next() else {
                return String::new();
            };

            let mut collapsed = String::with_capacity(trimmed.len());
            collapsed.push_str(first);
            for word in words {
                collapsed.push(' ');
                collapsed.push_str(word);
            }
            collapsed
        }
        None => String::new(),
    }
}

fn collapse_ascii_whitespace(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut in_ws = false;

    for &b in bytes {
        let is_ws = matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0C | 0x0B);
        if is_ws {
            if !in_ws {
                out.push(' ');
                in_ws = true;
            }
        } else {
            out.push(b as char);
            in_ws = false;
        }
    }

    out
}

fn strip_html_comments(s: &str) -> std::borrow::Cow<'_, str> {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    let mut segment_start = 0usize;
    let mut out: Option<String> = None;

    while i + 3 < bytes.len() {
        if bytes[i] == b'<' && bytes[i + 1] == b'!' && bytes[i + 2] == b'-' && bytes[i + 3] == b'-'
        {
            let mut end = i + 4;
            let mut found_close = false;
            while end + 2 < bytes.len() {
                if bytes[end] == b'-' && bytes[end + 1] == b'-' && bytes[end + 2] == b'>' {
                    found_close = true;
                    break;
                }
                end += 1;
            }

            if found_close {
                let buf = out.get_or_insert_with(|| String::with_capacity(s.len()));
                if segment_start < i {
                    buf.push_str(&s[segment_start..i]);
                }
                i = end + 3;
                segment_start = i;
                continue;
            }

            // Unclosed comment: behave like regex and leave tail unchanged.
            break;
        }
        i += 1;
    }

    if let Some(mut buf) = out {
        if segment_start < s.len() {
            buf.push_str(&s[segment_start..]);
        }
        std::borrow::Cow::Owned(buf)
    } else {
        std::borrow::Cow::Borrowed(s)
    }
}

fn strip_html_tags(s: &str) -> std::borrow::Cow<'_, str> {
    if !s.contains('<') {
        return std::borrow::Cow::Borrowed(s);
    }

    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut idx = 0usize;
    let mut segment_start = 0usize;

    while idx < bytes.len() {
        if bytes[idx] == b'<' {
            if segment_start < idx {
                out.push_str(&s[segment_start..idx]);
            }
            idx += 1;
            while idx < bytes.len() && bytes[idx] != b'>' {
                idx += 1;
            }
            if idx < bytes.len() {
                idx += 1;
            }
            segment_start = idx;
        } else {
            idx += 1;
        }
    }

    if segment_start < bytes.len() {
        out.push_str(&s[segment_start..]);
    }

    std::borrow::Cow::Owned(out)
}

fn decode_html_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }

    let mut decoded = String::with_capacity(s.len());
    let mut rest = s;

    while let Some(pos) = rest.find('&') {
        decoded.push_str(&rest[..pos]);
        rest = &rest[pos..];

        let Some(end) = rest.find(';') else {
            decoded.push_str(rest);
            return decoded;
        };

        let entity = &rest[..=end];
        match entity {
            "&amp;" => decoded.push('&'),
            "&#x27;" | "&#39;" => decoded.push('\''),
            "&quot;" => decoded.push('"'),
            "&lt;" => decoded.push('<'),
            "&gt;" => decoded.push('>'),
            "&nbsp;" => decoded.push(' '),
            _ => decoded.push_str(entity),
        }

        rest = &rest[end + 1..];
    }

    decoded.push_str(rest);
    decoded
}

fn parse_json_string_literal(raw: &str) -> Option<String> {
    if raw.len() >= 2
        && raw.as_bytes().first() == Some(&b'"')
        && raw.as_bytes().last() == Some(&b'"')
    {
        let inner = &raw[1..raw.len() - 1];
        if !inner.as_bytes().contains(&b'\\') {
            return Some(inner.to_string());
        }
    }
    serde_json::from_str::<String>(raw).ok()
}

fn extract_json_string_field(text: &str, field: &str) -> Option<String> {
    let mut found = None;
    for_each_json_field(text, |ks, ke, vs, ve| {
        let key = &text[ks..ke];
        let value = &text[vs..ve];
        if key == field {
            found = if value == "null" {
                None
            } else if value.starts_with('"') {
                parse_json_string_literal(value).filter(|s| !s.trim().is_empty())
            } else {
                None
            };
            return false;
        }
        true
    });
    found
}

fn for_each_json_field<F>(text: &str, mut f: F)
where
    F: FnMut(usize, usize, usize, usize) -> bool,
{
    let bytes = text.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] != b'"' {
            i += 1;
            continue;
        }

        let key_start = i + 1;
        let mut j = key_start;
        let mut escaped = false;
        let mut key_end = None;
        while j < bytes.len() {
            let b = bytes[j];
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                key_end = Some(j);
                break;
            }
            j += 1;
        }
        let Some(key_end) = key_end else {
            break;
        };

        let mut k = key_end + 1;
        while k < bytes.len() && bytes[k].is_ascii_whitespace() {
            k += 1;
        }
        if k >= bytes.len() || bytes[k] != b':' {
            i = key_end + 1;
            continue;
        }

        k += 1;
        while k < bytes.len() && bytes[k].is_ascii_whitespace() {
            k += 1;
        }
        if k >= bytes.len() {
            break;
        }

        let value_start = k;
        let value_end = if bytes[k] == b'"' {
            k += 1;
            let mut escaped = false;
            let mut end = None;
            while k < bytes.len() {
                let b = bytes[k];
                if escaped {
                    escaped = false;
                } else if b == b'\\' {
                    escaped = true;
                } else if b == b'"' {
                    end = Some(k + 1);
                    break;
                }
                k += 1;
            }
            match end {
                Some(end) => end,
                None => break,
            }
        } else if bytes[k].is_ascii_digit() || bytes[k] == b'-' || bytes[k] == b'.' {
            k += 1;
            while k < bytes.len()
                && (bytes[k].is_ascii_digit()
                    || matches!(bytes[k], b'.' | b'-' | b'+' | b'e' | b'E'))
            {
                k += 1;
            }
            k
        } else if bytes[k..].starts_with(b"null") || bytes[k..].starts_with(b"true") {
            k + 4
        } else if bytes[k..].starts_with(b"false") {
            k + 5
        } else {
            i = key_end + 1;
            continue;
        };

        if !f(key_start, key_end, value_start, value_end) {
            break;
        }
        i = value_end;
    }
}

fn extract_kap_link(text: &str) -> Option<String> {
    KAP_LINK_RE
        .captures(text)
        .and_then(|m| m.get(1))
        .map(|m| decode_html_entities(m.as_str()))
        .or_else(|| extract_json_string_field(text, "kapLink").map(|s| decode_html_entities(&s)))
        .filter(|s| !s.trim().is_empty())
}

fn parse_number_tr(s: Option<&str>) -> Option<f64> {
    let s = s?.trim();
    if s.is_empty() {
        return None;
    }
    let mut t = s.replace(['%', ' '], "");
    // Handle thousand separators and decimal comma
    if t.contains('.') && t.contains(',') {
        t = t.replace('.', "").replace(',', ".");
    } else if t.contains(',') {
        t = t.replace(',', ".");
    } else if t.contains('.') && THOUSANDS_RE.is_match(&t) {
        t = t.replace('.', "");
    }
    t.parse::<f64>().ok()
}

fn parse_i64_from_digits(text: &str) -> Option<i64> {
    if text.is_empty() {
        return None;
    }

    let mut value: i64 = 0;
    let mut seen_digit = false;

    for &b in text.as_bytes() {
        if b.is_ascii_digit() {
            seen_digit = true;
            let digit = (b - b'0') as i64;
            value = value.checked_mul(10)?.checked_add(digit)?;
        }
    }

    if seen_digit { Some(value) } else { None }
}

// Balanced bracket extractor. `start` and the returned end index are byte offsets.
fn parse_balanced(
    text: &str,
    start: usize,
    open_ch: char,
    close_ch: char,
) -> Option<(&str, usize)> {
    if start >= text.len()
        || !text.is_char_boundary(start)
        || text[start..].chars().next()? != open_ch
    {
        return None;
    }
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    let mut quote = '\0';

    for (offset, ch) in text[start..].char_indices() {
        let byte_idx = start + offset;
        if in_str {
            if esc {
                esc = false;
            } else if ch == '\\' {
                esc = true;
            } else if ch == quote {
                in_str = false;
            }
        } else {
            if ch == '"' || ch == '\'' {
                in_str = true;
                quote = ch;
            } else if ch == open_ch {
                depth += 1;
            } else if ch == close_ch {
                depth -= 1;
                if depth == 0 {
                    let end_exclusive = byte_idx + ch.len_utf8();
                    return Some((&text[start..end_exclusive], byte_idx));
                }
            }
        }
    }
    None
}

fn split_top_level_args(arg_text: &str) -> Vec<&str> {
    if arg_text.is_ascii() {
        return split_top_level_args_ascii(arg_text);
    }

    let mut args = Vec::new();
    let mut start = 0usize;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    let mut quote = '\0';

    for (idx, ch) in arg_text.char_indices() {
        if in_str {
            if esc {
                esc = false;
            } else if ch == '\\' {
                esc = true;
            } else if ch == quote {
                in_str = false;
            }
        } else {
            if ch == '"' || ch == '\'' {
                in_str = true;
                quote = ch;
            } else if ch == '(' || ch == '[' || ch == '{' {
                depth += 1;
            } else if ch == ')' || ch == ']' || ch == '}' {
                depth -= 1;
            } else if ch == ',' && depth == 0 {
                let segment = arg_text[start..idx].trim();
                if !segment.is_empty() {
                    args.push(segment);
                }
                start = idx + ch.len_utf8();
            }
        }
    }

    let last = arg_text[start..].trim();
    if !last.is_empty() {
        args.push(last);
    }
    args
}

fn split_top_level_args_ascii(arg_text: &str) -> Vec<&str> {
    let bytes = arg_text.as_bytes();
    let mut args = Vec::new();
    let mut start = 0usize;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    let mut quote = b'\0';
    let mut i = 0usize;

    while i < bytes.len() {
        let b = bytes[i];
        if in_str {
            if esc {
                esc = false;
            } else if b == b'\\' {
                esc = true;
            } else if b == quote {
                in_str = false;
            }
            i += 1;
            continue;
        }

        match b {
            b'"' | b'\'' => {
                in_str = true;
                quote = b;
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b',' if depth == 0 => {
                let segment = arg_text[start..i].trim();
                if !segment.is_empty() {
                    args.push(segment);
                }
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }

    let last = arg_text[start..].trim();
    if !last.is_empty() {
        args.push(last);
    }
    args
}

fn for_each_json_string_literal_filtered(
    text: &str,
    require_container: bool,
    mut callback: impl FnMut(String) -> bool,
) {
    let bytes = text.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] != b'"' {
            i += 1;
            continue;
        }

        let start = i;
        i += 1;
        let mut esc = false;
        let mut has_container_hint = false;
        while i < bytes.len() {
            let ch = bytes[i];
            if esc {
                esc = false;
            } else if ch == b'\\' {
                esc = true;
            } else if ch == b'{' || ch == b'[' {
                has_container_hint = true;
            } else if ch == b'"' {
                if !require_container || has_container_hint {
                    let literal = &text[start..i + 1];
                    if let Some(decoded) = parse_json_string_literal(literal)
                        && !decoded.trim().is_empty()
                        && !callback(decoded)
                    {
                        return;
                    }
                }
                break;
            }
            i += 1;
        }
        i += 1;
    }
}

fn extract_next_f_string_payloads(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut idx = 0usize;

    while let Some(pos) = text[idx..].find("__next_f.push") {
        let abs = idx + pos;
        let Some(open_rel) = text[abs..].find('(') else {
            idx = abs + 1;
            continue;
        };
        let open = abs + open_rel;
        let Some((slice, end)) = parse_balanced(text, open, '(', ')') else {
            idx = open + 1;
            continue;
        };

        let args_text = &slice[1..slice.len() - 1];
        for arg in split_top_level_args(args_text) {
            for_each_json_string_literal_filtered(arg, true, |decoded| {
                out.push(decoded);
                true
            });
        }

        idx = end + 1;
    }

    out
}

type RscTokenSpan = (usize, usize);

struct RscPayloadView<'a> {
    payload: &'a str,
}

impl<'a> RscPayloadView<'a> {
    fn new(payload: &'a str) -> Self {
        Self { payload }
    }

    fn parse_string_token(&self, span: Option<RscTokenSpan>) -> Option<String> {
        let (start, end) = span?;
        let token = &self.payload[start..end];
        if token == "null" || !token.starts_with('"') {
            return None;
        }
        parse_json_string_literal(token)
            .map(|s| decode_html_entities(&s))
            .filter(|s| !s.trim().is_empty())
    }

    fn parse_number_token(&self, span: Option<RscTokenSpan>) -> Option<f64> {
        let (start, end) = span?;
        let token = &self.payload[start..end];
        if token == "null" {
            return None;
        }
        if token.starts_with('"') {
            if let Some(decoded) = parse_json_string_literal(token) {
                return parse_number_tr(Some(&decoded));
            }
            return None;
        }
        token.parse::<f64>().ok()
    }
}

fn extract_rsc_fast_fields_from_payloads(payloads: &[String]) -> Map<String, Value> {
    let mut out = Map::new();

    for payload in payloads {
        let view = RscPayloadView::new(payload);

        let mut fon_kodu_tok: Option<RscTokenSpan> = None;
        let mut fund_code_tok: Option<RscTokenSpan> = None;
        let mut isin_kodu_tok: Option<RscTokenSpan> = None;
        let mut isin_tok: Option<RscTokenSpan> = None;
        let mut kap_link_tok: Option<RscTokenSpan> = None;
        let mut fon_toplam_deger_tok: Option<RscTokenSpan> = None;
        let mut fon_toplam_deger_tl_tok: Option<RscTokenSpan> = None;
        let mut fund_total_value_tok: Option<RscTokenSpan> = None;
        let mut pazar_payi_tok: Option<RscTokenSpan> = None;
        let mut market_share_tok: Option<RscTokenSpan> = None;
        let mut fon_risk_degeri_tok: Option<RscTokenSpan> = None;
        let mut risk_value_tok: Option<RscTokenSpan> = None;

        for_each_json_field(payload, |ks, ke, vs, ve| {
            let key = &payload[ks..ke];
            let span = (vs, ve);
            match key {
                "fonKodu" if fon_kodu_tok.is_none() => fon_kodu_tok = Some(span),
                "fundCode" if fund_code_tok.is_none() => fund_code_tok = Some(span),
                "isinKodu" if isin_kodu_tok.is_none() => isin_kodu_tok = Some(span),
                "isin" if isin_tok.is_none() => isin_tok = Some(span),
                "kapLink" if kap_link_tok.is_none() => kap_link_tok = Some(span),
                "fonToplamDeger" if fon_toplam_deger_tok.is_none() => {
                    fon_toplam_deger_tok = Some(span)
                }
                "fonToplamDegerTl" if fon_toplam_deger_tl_tok.is_none() => {
                    fon_toplam_deger_tl_tok = Some(span)
                }
                "fundTotalValue" if fund_total_value_tok.is_none() => {
                    fund_total_value_tok = Some(span)
                }
                "pazarPayi" if pazar_payi_tok.is_none() => pazar_payi_tok = Some(span),
                "marketShare" if market_share_tok.is_none() => market_share_tok = Some(span),
                "fonRiskDegeri" if fon_risk_degeri_tok.is_none() => {
                    fon_risk_degeri_tok = Some(span)
                }
                "riskValue" if risk_value_tok.is_none() => risk_value_tok = Some(span),
                _ => {}
            }
            true
        });

        if !out.contains_key("fon_kodu_raw")
            && let Some(code) = fon_kodu_tok
                .and_then(|span| view.parse_string_token(Some(span)))
                .or_else(|| fund_code_tok.and_then(|span| view.parse_string_token(Some(span))))
        {
            if !code.trim().is_empty() {
                out.insert("fon_kodu_raw".to_string(), Value::String(code.clone()));
                out.insert("fon_kodu".to_string(), Value::String(code));
            }
        }

        if !out.contains_key("isin_kodu_raw")
            && let Some(isin) = isin_kodu_tok
                .and_then(|span| view.parse_string_token(Some(span)))
                .or_else(|| isin_tok.and_then(|span| view.parse_string_token(Some(span))))
        {
            if !isin.trim().is_empty() {
                out.insert("isin_kodu_raw".to_string(), Value::String(isin.clone()));
                out.insert("isin_kodu".to_string(), Value::String(isin));
            }
        }

        if !out.contains_key("kap_bilgi_adresi")
            && let Some(link) = kap_link_tok
                .and_then(|span| view.parse_string_token(Some(span)))
                .filter(|s| !s.trim().is_empty())
        {
            out.insert(
                "kap_bilgi_adresi_raw".to_string(),
                Value::String(link.clone()),
            );
            out.insert("kap_bilgi_adresi".to_string(), Value::String(link));
        }

        if !out.contains_key("fon_toplam_deger_tl_raw") {
            let raw_total = fon_toplam_deger_tok
                .and_then(|span| view.parse_string_token(Some(span)))
                .or_else(|| {
                    fon_toplam_deger_tl_tok.and_then(|span| view.parse_string_token(Some(span)))
                })
                .or_else(|| fund_total_value_tok.and_then(|span| view.parse_string_token(Some(span))));
            if let Some(raw) = raw_total {
                out.insert(
                    "fon_toplam_deger_tl_raw".to_string(),
                    Value::String(raw.clone()),
                );
                if let Some(v) = parse_number_tr(Some(&raw))
                    && let Some(num) = serde_json::Number::from_f64(v)
                {
                    out.insert("fon_toplam_deger_tl".to_string(), Value::Number(num));
                }
            } else if let Some(v) = fon_toplam_deger_tok
                .and_then(|span| view.parse_number_token(Some(span)))
                .or_else(|| {
                    fon_toplam_deger_tl_tok.and_then(|span| view.parse_number_token(Some(span)))
                })
                .or_else(|| fund_total_value_tok.and_then(|span| view.parse_number_token(Some(span))))
                && let Some(num) = serde_json::Number::from_f64(v)
            {
                out.insert(
                    "fon_toplam_deger_tl_raw".to_string(),
                    Value::String(v.to_string()),
                );
                out.insert("fon_toplam_deger_tl".to_string(), Value::Number(num));
            }
        }

        if !out.contains_key("pazar_payi_raw_pct") {
            let raw_share = pazar_payi_tok
                .and_then(|span| view.parse_string_token(Some(span)))
                .or_else(|| market_share_tok.and_then(|span| view.parse_string_token(Some(span))));
            if let Some(raw) = raw_share {
                if let Some(v) = parse_number_tr(Some(&raw))
                    && let Some(num) = serde_json::Number::from_f64(v)
                {
                    out.insert("pazar_payi_raw_pct".to_string(), Value::Number(num));
                }
            } else if let Some(v) = pazar_payi_tok
                .and_then(|span| view.parse_number_token(Some(span)))
                .or_else(|| market_share_tok.and_then(|span| view.parse_number_token(Some(span))))
                && let Some(num) = serde_json::Number::from_f64(v)
            {
                out.insert("pazar_payi_raw_pct".to_string(), Value::Number(num));
            }
        }

        if !out.contains_key("fon_risk_degeri") {
            let raw_risk = fon_risk_degeri_tok
                .and_then(|span| view.parse_string_token(Some(span)))
                .or_else(|| risk_value_tok.and_then(|span| view.parse_string_token(Some(span))));
            if let Some(raw) = raw_risk {
                let mut num = String::new();
                for ch in raw.chars().skip_while(|c| c.is_ascii_whitespace()) {
                    if ch.is_ascii_digit() {
                        num.push(ch);
                    } else {
                        break;
                    }
                }
                if let Ok(n) = num.parse::<i64>() {
                    out.insert("fon_risk_degeri".to_string(), Value::Number(n.into()));
                }
            } else if let Some(v) = fon_risk_degeri_tok
                .and_then(|span| view.parse_number_token(Some(span)))
                .or_else(|| risk_value_tok.and_then(|span| view.parse_number_token(Some(span))))
            {
                out.insert(
                    "fon_risk_degeri".to_string(),
                    Value::Number((v as i64).into()),
                );
            }
        }

        if out.contains_key("fon_kodu")
            && out.contains_key("isin_kodu")
            && out.contains_key("kap_bilgi_adresi")
            && out.contains_key("fon_toplam_deger_tl")
            && out.contains_key("pazar_payi_raw_pct")
            && out.contains_key("fon_risk_degeri")
        {
            break;
        }
    }

    out
}

fn format_tr_thousands(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(ch);
    }
    if n < 0 { format!("-{out}") } else { out }
}

fn enrich_rsc_profile_from_payloads(payloads: &[String], out: &mut Map<String, Value>) {
    for payload in payloads {
        let view = RscPayloadView::new(payload);

        let mut tefas_durum_tok: Option<RscTokenSpan> = None;
        let mut platform_status_tok: Option<RscTokenSpan> = None;
        let mut bas_is_saat_tok: Option<RscTokenSpan> = None;
        let mut trade_start_tok: Option<RscTokenSpan> = None;
        let mut son_is_saat_tok: Option<RscTokenSpan> = None;
        let mut trade_end_tok: Option<RscTokenSpan> = None;
        let mut fon_geri_alis_valor_tok: Option<RscTokenSpan> = None;
        let mut fund_purchase_value_tok: Option<RscTokenSpan> = None;
        let mut fon_satis_valor_tok: Option<RscTokenSpan> = None;
        let mut fund_sale_value_tok: Option<RscTokenSpan> = None;
        let mut min_alis_tok: Option<RscTokenSpan> = None;
        let mut min_purchase_tok: Option<RscTokenSpan> = None;
        let mut min_satis_tok: Option<RscTokenSpan> = None;
        let mut min_sale_tok: Option<RscTokenSpan> = None;
        let mut max_alis_tok: Option<RscTokenSpan> = None;
        let mut max_purchase_tok: Option<RscTokenSpan> = None;
        let mut max_satis_tok: Option<RscTokenSpan> = None;
        let mut max_sale_tok: Option<RscTokenSpan> = None;
        let mut yatirimci_sayi_tok: Option<RscTokenSpan> = None;
        let mut investor_count_tok: Option<RscTokenSpan> = None;
        let mut fon_kategori_tok: Option<RscTokenSpan> = None;
        let mut fund_category_tok: Option<RscTokenSpan> = None;
        let mut kategori_derece_tok: Option<RscTokenSpan> = None;
        let mut fund_category_degree_tok: Option<RscTokenSpan> = None;
        let mut kategori_fon_say_tok: Option<RscTokenSpan> = None;
        let mut giris_kom_tok: Option<RscTokenSpan> = None;
        let mut entrance_commission_tok: Option<RscTokenSpan> = None;
        let mut cikis_kom_tok: Option<RscTokenSpan> = None;
        let mut exit_commission_tok: Option<RscTokenSpan> = None;
        let mut faiz_icerigi_tok: Option<RscTokenSpan> = None;
        let mut interest_content_tok: Option<RscTokenSpan> = None;

        for_each_json_field(payload, |ks, ke, vs, ve| {
            let key = &payload[ks..ke];
            let span = (vs, ve);
            match key {
                "tefasDurum" if tefas_durum_tok.is_none() => tefas_durum_tok = Some(span),
                "platformStatus" if platform_status_tok.is_none() => {
                    platform_status_tok = Some(span)
                }
                "basIsSaat" if bas_is_saat_tok.is_none() => bas_is_saat_tok = Some(span),
                "tradeStartTime" if trade_start_tok.is_none() => trade_start_tok = Some(span),
                "sonIsSaat" if son_is_saat_tok.is_none() => son_is_saat_tok = Some(span),
                "tradeEndTime" if trade_end_tok.is_none() => trade_end_tok = Some(span),
                "fonGeriAlisValor" if fon_geri_alis_valor_tok.is_none() => {
                    fon_geri_alis_valor_tok = Some(span)
                }
                "fundPurchaseValue" if fund_purchase_value_tok.is_none() => {
                    fund_purchase_value_tok = Some(span)
                }
                "fonSatisValor" if fon_satis_valor_tok.is_none() => {
                    fon_satis_valor_tok = Some(span)
                }
                "fundSaleValue" if fund_sale_value_tok.is_none() => {
                    fund_sale_value_tok = Some(span)
                }
                "minAlis" if min_alis_tok.is_none() => min_alis_tok = Some(span),
                "minPurchaseAmount" if min_purchase_tok.is_none() => min_purchase_tok = Some(span),
                "minSatis" if min_satis_tok.is_none() => min_satis_tok = Some(span),
                "minSaleAmount" if min_sale_tok.is_none() => min_sale_tok = Some(span),
                "maxAlis" if max_alis_tok.is_none() => max_alis_tok = Some(span),
                "maxPurchaseAmount" if max_purchase_tok.is_none() => max_purchase_tok = Some(span),
                "maxSatis" if max_satis_tok.is_none() => max_satis_tok = Some(span),
                "maxSaleAmount" if max_sale_tok.is_none() => max_sale_tok = Some(span),
                "yatirimciSayi" if yatirimci_sayi_tok.is_none() => yatirimci_sayi_tok = Some(span),
                "investorCount" if investor_count_tok.is_none() => investor_count_tok = Some(span),
                "fonKategori" if fon_kategori_tok.is_none() => fon_kategori_tok = Some(span),
                "fundCategory" if fund_category_tok.is_none() => fund_category_tok = Some(span),
                "kategoriDerece" if kategori_derece_tok.is_none() => {
                    kategori_derece_tok = Some(span)
                }
                "fundCategoryDegree" if fund_category_degree_tok.is_none() => {
                    fund_category_degree_tok = Some(span)
                }
                "kategoriFonSay" if kategori_fon_say_tok.is_none() => {
                    kategori_fon_say_tok = Some(span)
                }
                "girisKomisyonu" if giris_kom_tok.is_none() => giris_kom_tok = Some(span),
                "entranceCommission" if entrance_commission_tok.is_none() => {
                    entrance_commission_tok = Some(span)
                }
                "cikisKomisyonu" if cikis_kom_tok.is_none() => cikis_kom_tok = Some(span),
                "exitCommission" if exit_commission_tok.is_none() => {
                    exit_commission_tok = Some(span)
                }
                "faizIcerigi" if faiz_icerigi_tok.is_none() => faiz_icerigi_tok = Some(span),
                "interestContent" if interest_content_tok.is_none() => {
                    interest_content_tok = Some(span)
                }
                _ => {}
            }
            true
        });

        if !out.contains_key("platform_islem_durumu_raw")
            && let Some(v) = view
                .parse_string_token(tefas_durum_tok)
                .or_else(|| view.parse_string_token(platform_status_tok))
            && !v.is_empty()
        {
            out.insert(
                "platform_islem_durumu_raw".to_string(),
                Value::String(v.clone()),
            );
            out.insert("platform_islem_durumu".to_string(), Value::String(v));
        }

        if !out.contains_key("islem_baslangic_saati_raw")
            && let Some(v) = view
                .parse_string_token(bas_is_saat_tok)
                .or_else(|| view.parse_string_token(trade_start_tok))
            && !v.is_empty()
        {
            out.insert(
                "islem_baslangic_saati_raw".to_string(),
                Value::String(v.clone()),
            );
            out.insert("islem_baslangic_saati".to_string(), Value::String(v));
        }

        if !out.contains_key("son_islem_saati_raw")
            && let Some(v) = view
                .parse_string_token(son_is_saat_tok)
                .or_else(|| view.parse_string_token(trade_end_tok))
            && !v.is_empty()
        {
            out.insert("son_islem_saati_raw".to_string(), Value::String(v.clone()));
            out.insert("son_islem_saati".to_string(), Value::String(v));
        }

        let mut insert_i64 = |base: &str, candidates: &[&str]| {
            if out.contains_key(base) {
                return;
            }
            let v = match candidates {
                ["fonGeriAlisValor", "fundPurchaseValue"] => {
                    view.parse_number_token(fon_geri_alis_valor_tok)
                        .or_else(|| view.parse_number_token(fund_purchase_value_tok))
                }
                ["fonSatisValor", "fundSaleValue"] => {
                    view.parse_number_token(fon_satis_valor_tok)
                        .or_else(|| view.parse_number_token(fund_sale_value_tok))
                }
                ["minAlis", "minPurchaseAmount"] => {
                    view.parse_number_token(min_alis_tok)
                        .or_else(|| view.parse_number_token(min_purchase_tok))
                }
                ["minSatis", "minSaleAmount"] => {
                    view.parse_number_token(min_satis_tok)
                        .or_else(|| view.parse_number_token(min_sale_tok))
                }
                ["maxAlis", "maxPurchaseAmount"] => {
                    view.parse_number_token(max_alis_tok)
                        .or_else(|| view.parse_number_token(max_purchase_tok))
                }
                ["maxSatis", "maxSaleAmount"] => {
                    view.parse_number_token(max_satis_tok)
                        .or_else(|| view.parse_number_token(max_sale_tok))
                }
                ["yatirimciSayi", "investorCount"] => {
                    view.parse_number_token(yatirimci_sayi_tok)
                        .or_else(|| view.parse_number_token(investor_count_tok))
                }
                _ => None,
            };
            if let Some(v) = v {
                let n = v.trunc() as i64;
                out.insert(format!("{}_raw", base), Value::String(n.to_string()));
                out.insert(base.to_string(), Value::Number(n.into()));
            }
        };

        insert_i64(
            "fon_alis_valoru",
            &["fonGeriAlisValor", "fundPurchaseValue"],
        );
        insert_i64("fon_satis_valoru", &["fonSatisValor", "fundSaleValue"]);
        insert_i64("min_alis_islem_miktari", &["minAlis", "minPurchaseAmount"]);
        insert_i64("min_satis_islem_miktari", &["minSatis", "minSaleAmount"]);
        insert_i64("max_alis_islem_miktari", &["maxAlis", "maxPurchaseAmount"]);
        insert_i64("max_satis_islem_miktari", &["maxSatis", "maxSaleAmount"]);
        insert_i64("yatirimci_sayisi", &["yatirimciSayi", "investorCount"]);

        if !out.contains_key("kategori_raw")
            && let Some(v) = view
                .parse_string_token(fon_kategori_tok)
                .or_else(|| view.parse_string_token(fund_category_tok))
            && !v.is_empty()
        {
            out.insert("kategori_raw".to_string(), Value::String(v.clone()));
            out.insert("kategori".to_string(), Value::String(v));
        }

        if !out.contains_key("son_1yillik_kategori_derecesi_raw")
            && !out.contains_key("son_1yillik_kategori_derecesi")
            && let Some(degree) = view
                .parse_number_token(kategori_derece_tok)
                .or_else(|| view.parse_number_token(fund_category_degree_tok))
        {
            let rank = degree.trunc() as i64;
            let raw = if let Some(total) = view.parse_number_token(kategori_fon_say_tok) {
                format!("{}/{}", rank, format_tr_thousands(total.trunc() as i64))
            } else {
                rank.to_string()
            };
            out.insert(
                "son_1yillik_kategori_derecesi_raw".to_string(),
                Value::String(raw),
            );
            out.insert(
                "son_1yillik_kategori_derecesi".to_string(),
                Value::Number(rank.into()),
            );
        }

        if !out.contains_key("giris_komisyonu_raw")
            && let Some(v) = view
                .parse_string_token(giris_kom_tok)
                .or_else(|| view.parse_string_token(entrance_commission_tok))
        {
            out.insert("giris_komisyonu_raw".to_string(), Value::String(v));
        }

        if !out.contains_key("cikis_komisyonu_raw")
            && let Some(v) = view
                .parse_string_token(cikis_kom_tok)
                .or_else(|| view.parse_string_token(exit_commission_tok))
        {
            out.insert("cikis_komisyonu_raw".to_string(), Value::String(v));
        }

        if !out.contains_key("fon_faiz_icerigi_raw")
            && let Some(v) = view
                .parse_string_token(faiz_icerigi_tok)
                .or_else(|| view.parse_string_token(interest_content_tok))
        {
            out.insert("fon_faiz_icerigi_raw".to_string(), Value::String(v));
        }
    }
}

fn try_parse(block: &str) -> Option<Value> {
    if block.trim().is_empty() {
        return None;
    }
    // Try strict JSON first
    if let Ok(v) = serde_json::from_str::<Value>(block) {
        return Some(v);
    }
    // Fallback: remove trailing commas before ] or }
    let cleaned = TRAILING_COMMA_RE.replace_all(block, "$1");
    if let Ok(v) = serde_json::from_str::<Value>(&cleaned) {
        return Some(v);
    }
    None
}

fn find_series_blocks(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut idx = 0usize;
    while let Some(m) = SERIES_KEY_RE.find_at(text, idx) {
        let pos = m.start();
        let colon = text[pos..].find(':');
        if colon.is_none() {
            idx = pos + 6;
            continue;
        }
        let colon = pos + colon.unwrap();
        let bracket = text[colon..].find('[');
        if bracket.is_none() {
            idx = pos + 6;
            continue;
        }
        let bracket = colon + bracket.unwrap();
        if let Some((slice, end)) = parse_balanced(text, bracket, '[', ']') {
            out.push(slice.to_owned());
            idx = end + 1;
        } else {
            idx = bracket + 1;
        }
    }
    out
}

fn find_xaxis_blocks(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut idx = 0usize;
    while let Some(m) = XAXIS_KEY_RE.find_at(text, idx) {
        let pos = m.start();
        let colon = text[pos..].find(':');
        if colon.is_none() {
            idx = pos + 5;
            continue;
        }
        let colon = pos + colon.unwrap();
        let bracket = text[colon..].find('[');
        if bracket.is_none() {
            idx = pos + 5;
            continue;
        }
        let bracket = colon + bracket.unwrap();
        if let Some((slice, end)) = parse_balanced(text, bracket, '[', ']') {
            out.push(slice.to_owned());
            idx = end + 1;
        } else {
            idx = bracket + 1;
        }
    }
    out
}

fn next_p_value_after_label(hay: &str, label: &str, prefer_digits: bool) -> Option<String> {
    let label_end = hay.find(label)? + label.len();
    let after = &hay[label_end..];
    let close = after.find("</p>")?;
    let search_after = &after[close + 4..];
    next_p_value_in_fragment(search_after, prefer_digits)
}

fn next_p_value_after_any_label(hay: &str, labels: &[&str], prefer_digits: bool) -> Option<String> {
    labels
        .iter()
        .find_map(|label| next_p_value_after_label(hay, label, prefer_digits))
}

fn collect_next_p_values_after_labels(
    hay: &str,
    labels: &[&'static str],
    prefer_digits: bool,
) -> std::collections::HashMap<&'static str, String> {
    let mut values = std::collections::HashMap::with_capacity(labels.len());

    for mat in FALLBACK_LABEL_MATCHER.find_iter(hay) {
        let label = labels[mat.pattern().as_usize()];
        if values.contains_key(label) {
            continue;
        }

        let after = &hay[mat.start()..];
        let Some(close) = after.find("</p>") else {
            continue;
        };
        let search_after = &after[close + 4..];
        if let Some(value) = next_p_value_in_fragment(search_after, prefer_digits) {
            values.insert(label, value);
        }
    }

    values
}

fn next_p_value_in_fragment(fragment: &str, prefer_digits: bool) -> Option<String> {
    let mut idx = 0usize;
    let mut first_value: Option<String> = None;

    for _ in 0..6 {
        let Some(open_rel) = fragment[idx..].find("<p") else {
            break;
        };
        let open_idx = idx + open_rel;
        let Some(gt_rel) = fragment[open_idx..].find('>') else {
            break;
        };
        let content_start = open_idx + gt_rel + 1;
        let Some(close_rel) = fragment[content_start..].find("</p>") else {
            break;
        };
        let content_end = content_start + close_rel;

        let raw = &fragment[content_start..content_end];
        let no_tags = strip_html_tags(raw);
        let val = clean_whitespace(Some(&no_tags));
        if !val.is_empty() {
            if first_value.is_none() {
                first_value = Some(val.clone());
            }
            if !prefer_digits || val.chars().any(|c| c.is_ascii_digit()) {
                return Some(val);
            }
        }

        idx = content_end + 4;
        if idx >= fragment.len() {
            break;
        }
    }

    first_value
}

fn normalize_data(v: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    if let Value::Array(arr) = v {
        for entry in arr {
            match entry {
                Value::Array(a) if a.len() >= 2 => {
                    let name = a[0].clone();
                    let value = a[1].clone();
                    out.push(json!({"name": name, "value": value}));
                }
                Value::Object(obj) => {
                    let name = obj
                        .get("name")
                        .or_else(|| obj.get("label"))
                        .cloned()
                        .unwrap_or(Value::Null);
                    let value = obj
                        .get("y")
                        .or_else(|| obj.get("value"))
                        .cloned()
                        .unwrap_or(Value::Null);
                    out.push(json!({"name": name, "value": value}));
                }
                _ => {}
            }
        }
    }
    out
}

fn extract_allocation_from_highcharts_config(conf: &Value) -> Vec<Value> {
    if !conf.is_object() {
        return vec![];
    }
    if let Some(series) = conf.get("series")
        && let Value::Array(arr) = series
    {
        for s in arr {
            if let Value::Object(map) = s {
                let typ = map.get("type").and_then(|v| v.as_str()).unwrap_or("");
                let name = map
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_lowercase();
                if typ == "pie"
                    || name.contains("varl")
                    || name.contains("dağılım")
                    || name.contains("dagilim")
                {
                    return normalize_data(map.get("data").unwrap_or(&Value::Null));
                }
            }
        }
        for s in arr {
            if let Some(data) = s.get("data") {
                return normalize_data(data);
            }
        }
    }
    if let Some(plot) = conf.get("plotOptions")
        && let Some(pie) = plot.get("pie")
    {
        return normalize_data(pie.get("data").unwrap_or(&Value::Null));
    }
    vec![]
}

fn extract_fund_name(text: &str) -> Value {
    if text.contains("MainContent_FormViewMainIndicators_LabelFund")
        && let Some(m) = FUND_NAME_ID_RE.captures(text)
    {
        Value::String(clean_whitespace(m.get(1).map(|x| x.as_str())).to_string())
    } else if let Some(h6) = H6_RE.captures(text) {
        Value::String(clean_whitespace(h6.get(1).map(|x| x.as_str())).to_string())
    } else if let Some(title) = extract_title(text) {
        let code = title.split(" - ").next().map(|s| s.trim()).unwrap_or("");
        if !code.is_empty() {
            Value::String(code.to_string())
        } else {
            Value::Null
        }
    } else {
        Value::Null
    }
}

fn extract_indicator_pairs(text: &str) -> std::collections::HashMap<String, String> {
    let block = if let Some(m) = MAIN_INDICATORS_RE.captures(text) {
        m.get(1).map(|x| x.as_str()).unwrap_or(text)
    } else {
        text
    };

    let mut pairs = std::collections::HashMap::new();
    for li in LI_RE.captures_iter(block) {
        let li_html = li.get(1).map(|x| x.as_str()).unwrap_or("");
        if let Some(mm) = LI_LABEL_RE.captures(li_html) {
            let label = clean_whitespace(mm.get(1).map(|x| x.as_str()));
            let span_val = SPAN_RE
                .captures(li_html)
                .and_then(|s| s.get(1))
                .map(|g| clean_whitespace(Some(g.as_str())))
                .unwrap_or_default();
            pairs.insert(label, span_val);
        }
    }
    pairs
}

fn normalize_label(s: &str) -> String {
    if !s.contains('&') && s.is_ascii() {
        return s.to_ascii_lowercase();
    }

    let decoded = decode_html_entities(s);
    if decoded.is_ascii() {
        return decoded.to_ascii_lowercase();
    }

    let mut normalized = String::with_capacity(decoded.len());

    for ch in decoded.chars() {
        match ch {
            'ı' | 'İ' => normalized.push('i'),
            'ş' | 'Ş' => normalized.push('s'),
            'ğ' | 'Ğ' => normalized.push('g'),
            'ü' | 'Ü' => normalized.push('u'),
            'ö' | 'Ö' => normalized.push('o'),
            'ç' | 'Ç' => normalized.push('c'),
            other => normalized.extend(other.to_lowercase()),
        }
    }

    normalized
}

fn build_normalized_pairs(
    pairs: &std::collections::HashMap<String, String>,
) -> std::collections::HashMap<String, String> {
    let mut normalized = std::collections::HashMap::with_capacity(pairs.len());
    for (label, value) in pairs {
        normalized
            .entry(normalize_label(label))
            .or_insert_with(|| value.clone());
    }
    normalized
}

fn extract_p_pairs(text: &str) -> std::collections::HashMap<String, String> {
    let mut p_pairs = std::collections::HashMap::new();

    // Single-pass preferred path: capture explicit <p>label</p><p>value</p> pairs.
    for cap in P_PAIR_RE.captures_iter(text) {
        let label = clean_whitespace(cap.get(1).map(|x| x.as_str()));
        let value = clean_whitespace(cap.get(2).map(|x| x.as_str()));
        if value != label && !value.is_empty() && label.len() <= 64 {
            p_pairs.entry(label).or_insert_with(|| value.clone());
        }
    }

    // Fallback for pages where markup is not strictly paired.
    if p_pairs.is_empty() {
        let p_texts = extract_p_texts(text);

        for window in p_texts.windows(2) {
            if window.len() < 2 {
                continue;
            }
            let label = window[0].clone();
            let value = window[1].clone();
            if value != label && !value.is_empty() && label.len() <= 64 {
                p_pairs.entry(label).or_insert_with(|| value.clone());
            }
        }
    }

    p_pairs
}

fn extract_p_texts(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;

    while i + 2 < bytes.len() {
        if bytes[i] == b'<' && (bytes[i + 1] == b'p' || bytes[i + 1] == b'P') {
            // Advance until end of opening <p ...> tag.
            let mut open_end = i + 2;
            while open_end < bytes.len() && bytes[open_end] != b'>' {
                open_end += 1;
            }
            if open_end >= bytes.len() {
                break;
            }

            let content_start = open_end + 1;

            // Find the next closing </p>.
            let mut j = content_start;
            let mut close_pos: Option<usize> = None;
            while j + 3 < bytes.len() {
                if bytes[j] == b'<'
                    && bytes[j + 1] == b'/'
                    && (bytes[j + 2] == b'p' || bytes[j + 2] == b'P')
                    && bytes[j + 3] == b'>'
                {
                    close_pos = Some(j);
                    break;
                }
                j += 1;
            }

            if let Some(close_idx) = close_pos {
                let raw = &text[content_start..close_idx];
                let no_tags = strip_html_tags(raw);
                let cleaned = clean_whitespace(Some(&no_tags));
                if !cleaned.is_empty() {
                    out.push(cleaned);
                }
                i = close_idx + 4;
                continue;
            }

            break;
        }
        i += 1;
    }

    out
}

struct PLabelScanner<'a> {
    text: &'a str,
}

impl<'a> PLabelScanner<'a> {
    fn new(text: &'a str) -> Self {
        Self { text }
    }

    fn extract_all_pairs(&self) -> std::collections::HashMap<String, String> {
        extract_p_pairs(self.text)
    }

    fn find_value_after_label(&self, label: &str, prefer_digits: bool) -> Option<String> {
        next_p_value_after_label(self.text, label, prefer_digits)
    }

    fn collect_values_after_labels(
        &self,
        labels: &[&'static str],
        prefer_digits: bool,
    ) -> std::collections::HashMap<&'static str, String> {
        collect_next_p_values_after_labels(self.text, labels, prefer_digits)
    }
}

struct PLabelLookup<'a> {
    scanner: &'a PLabelScanner<'a>,
    pairs: &'a std::collections::HashMap<String, String>,
    p_pairs: &'a std::collections::HashMap<String, String>,
    p_pairs_norm: &'a std::cell::OnceCell<std::collections::HashMap<String, String>>,
    fallback_label_values: &'a std::cell::OnceCell<std::collections::HashMap<&'static str, String>>,
}

impl<'a> PLabelLookup<'a> {
    fn new(
        scanner: &'a PLabelScanner<'a>,
        pairs: &'a std::collections::HashMap<String, String>,
        p_pairs: &'a std::collections::HashMap<String, String>,
        p_pairs_norm: &'a std::cell::OnceCell<std::collections::HashMap<String, String>>,
        fallback_label_values: &'a std::cell::OnceCell<
            std::collections::HashMap<&'static str, String>,
        >,
    ) -> Self {
        Self {
            scanner,
            pairs,
            p_pairs,
            p_pairs_norm,
            fallback_label_values,
        }
    }

    fn find_label(&self, label: &str) -> Option<String> {
        if let Some(v) = self.pairs.get(label) {
            return Some(v.clone());
        }
        if let Some(v) = self.p_pairs.get(label) {
            return Some(v.clone());
        }

        let label_norm = normalize_label(label);
        if let Some(v) = self
            .p_pairs_norm
            .get_or_init(|| build_normalized_pairs(self.p_pairs))
            .get(&label_norm)
        {
            return Some(v.clone());
        }

        if FALLBACK_LABELS.contains(&label) {
            let fallback_values = self.fallback_label_values.get_or_init(|| {
                self.scanner
                    .collect_values_after_labels(FALLBACK_LABELS, true)
            });
            if let Some(v) = fallback_values.get(label) {
                return Some(v.clone());
            }
        }

        self.scanner.find_value_after_label(label, true)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DocumentKind {
    LegacyHtml,
    NextJsRsc,
    Hybrid,
}

struct FundDocument<'a> {
    kind: DocumentKind,
    full_text: &'a str,
    _info_scope: Option<&'a str>,
    chart_scope: &'a str,
}

impl<'a> FundDocument<'a> {
    fn classify(text: &'a str) -> Self {
        let info_scope = if let Some(m) = INFO_SCOPE_RE.find(text) {
            let start = m.start();
            let end = std::cmp::min(text.len(), start + 8000);
            Some(&text[start..end])
        } else {
            None
        };

        let chart_scope = if let Some(pos) = text.find("Highcharts") {
            &text[pos..]
        } else {
            text
        };

        let has_rsc_frames = text.contains("__next_f.push");
        let kind = match (has_rsc_frames, info_scope.is_some()) {
            (false, _) => DocumentKind::LegacyHtml,
            (true, false) => DocumentKind::NextJsRsc,
            (true, true) => DocumentKind::Hybrid,
        };

        Self {
            kind,
            full_text: text,
            _info_scope: info_scope,
            chart_scope,
        }
    }
}

struct ParseAccumulator {
    fields: Map<String, Value>,
}

impl ParseAccumulator {
    fn new() -> Self {
        Self { fields: Map::new() }
    }

    fn insert_value(&mut self, key: &str, value: Value) {
        self.fields.insert(key.to_string(), value);
    }

    fn insert_decoded_optional_string(&mut self, key: &str, value: Option<String>) {
        self.insert_value(
            key,
            value
                .map(|s| Value::String(decode_html_entities(&s)))
                .unwrap_or(Value::Null),
        );
    }

    fn insert_optional_f64(&mut self, key: &str, value: Option<f64>) {
        self.insert_value(
            key,
            value
                .and_then(serde_json::Number::from_f64)
                .map(Value::Number)
                .unwrap_or(Value::Null),
        );
    }
}

impl std::ops::Deref for ParseAccumulator {
    type Target = Map<String, Value>;

    fn deref(&self) -> &Self::Target {
        &self.fields
    }
}

impl std::ops::DerefMut for ParseAccumulator {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fields
    }
}

fn insert_return_metric(
    fields: &mut Map<String, Value>,
    raw_key: &str,
    pct_key: &str,
    value: Option<String>,
) {
    fields.insert(
        raw_key.to_string(),
        value
            .clone()
            .map_or(Value::Null, |s| Value::String(decode_html_entities(&s))),
    );

    if let Some(v) = parse_number_tr(value.as_deref()) {
        fields.insert(
            pct_key.to_string(),
            Value::Number(serde_json::Number::from_f64(v).unwrap()),
        );
    }
}

fn capture_return_metrics<F>(
    fields: &mut Map<String, Value>,
    text: &str,
    find_label: &F,
    include_targeted_fallback: bool,
) where
    F: Fn(&str) -> Option<String>,
{
    for (label, raw_key, pct_key) in RETURN_SPECS {
        let value = find_label(label).or_else(|| {
            include_targeted_fallback.then(|| next_p_value_after_label(text, label, true))?
        });
        insert_return_metric(fields, raw_key, pct_key, value);
    }
}

#[cfg(not(feature = "return_single_pass"))]
fn fill_missing_return_metric_fallbacks(fields: &mut Map<String, Value>, text: &str) {
    for (label, raw_key, pct_key) in RETURN_SPECS {
        if (!fields.contains_key(*raw_key) || matches!(fields.get(*raw_key), Some(Value::Null)))
            && let Some(value) = next_p_value_after_label(text, label, true)
            && !value.is_empty()
        {
            fields.insert(
                raw_key.to_string(),
                Value::String(decode_html_entities(&value)),
            );
            if let Some(number) = parse_number_tr(Some(&value)) {
                fields.insert(
                    pct_key.to_string(),
                    Value::Number(serde_json::Number::from_f64(number).unwrap()),
                );
            }
        }
    }
}

pub fn parse_html_text(text: &str) -> (Value, Value) {
    let doc = FundDocument::classify(text);
    let text = doc.full_text;
    let mut res = ParseAccumulator::new();

    // Fund name
    res.insert_value("fon_adi", extract_fund_name(text));

    // RSC detection: Next.js RSC pages embed all fund data as JSON inside
    // __next_f.push(<script>) payloads — no visible HTML label/value pairs exist.
    // Skip the three expensive regex scans that find nothing on such pages.
    let is_rsc = matches!(doc.kind, DocumentKind::NextJsRsc | DocumentKind::Hybrid);

    // main indicators block (skipped for RSC pages)
    let pairs = if is_rsc {
        std::collections::HashMap::new()
    } else {
        extract_indicator_pairs(text)
    };
    let p_label_scanner = PLabelScanner::new(text);
    let p_pairs = if matches!(doc.kind, DocumentKind::NextJsRsc) {
        std::collections::HashMap::new()
    } else {
        p_label_scanner.extract_all_pairs()
    };
    let p_pairs_norm = std::cell::OnceCell::new();

    let fallback_label_values = std::cell::OnceCell::new();
    let label_lookup = PLabelLookup::new(
        &p_label_scanner,
        &pairs,
        &p_pairs,
        &p_pairs_norm,
        &fallback_label_values,
    );

    let find_label = |label: &str| -> Option<String> { label_lookup.find_label(label) };

    // numeric/top-level indicators
    let son_fiyat_tl_raw = find_label("Son Fiyat (TL)");
    res.insert_decoded_optional_string("son_fiyat_tl_raw", son_fiyat_tl_raw.clone());
    res.insert_optional_f64("son_fiyat_tl", parse_number_tr(son_fiyat_tl_raw.as_deref()));

    let g = find_label("Günlük Getiri (%)");
    res.insert_decoded_optional_string("gunluk_getiri_raw", g.clone());
    res.insert_optional_f64("gunluk_getiri_pct", parse_number_tr(g.as_deref()));

    let pay_adet_raw = find_label("Pay (Adet)");
    res.insert_decoded_optional_string("pay_adet_raw", pay_adet_raw.clone());
    if let Some(p) = &pay_adet_raw {
        if let Some(n) = parse_i64_from_digits(p) {
            res.insert("pay_adet".to_string(), Value::Number(n.into()));
        } else {
            res.insert("pay_adet".to_string(), Value::Null);
        }
    } else {
        res.insert("pay_adet".to_string(), Value::Null);
    }

    let mut apply_profile_value = |k: &str, right: &str, ah: Option<&str>| {
        let decoded = decode_html_entities(right);
        if k != "fon_risk_degeri" && k != "pazar_payi_raw_pct" {
            res.insert(format!("{}_raw", k), Value::String(decoded.clone()));
        }
        match k {
            "fon_alis_valoru" | "fon_satis_valoru" => {
                if let Some(n) = parse_i64_from_digits(right) {
                    res.insert(k.to_string(), Value::Number(n.into()));
                } else if let Some(v) = parse_number_tr(Some(right)) {
                    let n = v.trunc() as i64;
                    res.insert(k.to_string(), Value::Number(n.into()));
                } else {
                    res.insert(k.to_string(), Value::Null);
                }
            }
            "min_alis_islem_miktari"
            | "min_satis_islem_miktari"
            | "max_alis_islem_miktari"
            | "max_satis_islem_miktari"
                if !right.is_empty() =>
            {
                if let Some(n) = parse_i64_from_digits(right) {
                    res.insert(k.to_string(), Value::Number(n.into()));
                }
            }
            "fon_toplam_deger_tl" => {
                if let Some(n) = parse_number_tr(Some(right)) {
                    res.insert(
                        k.to_string(),
                        Value::Number(serde_json::Number::from_f64(n).unwrap()),
                    );
                }
            }
            "pazar_payi_raw_pct" => {
                if let Some(n) = parse_number_tr(Some(right)) {
                    res.insert(
                        k.to_string(),
                        Value::Number(serde_json::Number::from_f64(n).unwrap()),
                    );
                }
            }
            "kategori" => {
                res.insert(k.to_string(), Value::String(decoded.clone()));
            }
            "son_1yillik_kategori_derecesi" => {
                let first_digits: String =
                    decoded.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(v) = first_digits.parse::<i64>() {
                    res.insert(k.to_string(), Value::Number(serde_json::Number::from(v)));
                }
            }
            "fon_risk_degeri" if !right.is_empty() => {
                let mut num = String::new();
                for ch in right.chars().skip_while(|c| c.is_ascii_whitespace()) {
                    if ch.is_ascii_digit() {
                        num.push(ch);
                    } else {
                        break;
                    }
                }
                if let Ok(n) = num.parse::<i64>() {
                    res.insert(k.to_string(), Value::Number(n.into()));
                    return;
                }
                if let Some(n) = parse_i64_from_digits(right) {
                    res.insert(k.to_string(), Value::Number(n.into()));
                }
            }
            "kap_bilgi_adresi" => {
                res.insert(
                    k.to_string(),
                    ah.map_or(Value::String(decoded), |h| Value::String(h.to_string())),
                );
            }
            _ => {}
        }
    };

    // Try old-style table extraction first
    if text.contains("MainContent_DetailsViewFund")
        && let Some(m) = DETAILS_VIEW_TABLE_RE.captures(text)
    {
        let tbl = m.get(1).map(|x| x.as_str()).unwrap_or("");
        for tr in TR_RE.captures_iter(tbl) {
            let tr_html = tr.get(1).map(|x| x.as_str()).unwrap_or("");
            let tds: Vec<String> = CELL_RE
                .captures_iter(tr_html)
                .filter_map(|c| c.get(1).map(|g| clean_whitespace(Some(g.as_str()))))
                .collect();
            if tds.is_empty() {
                continue;
            }
            let left = tds.first().cloned().unwrap_or_default();
            let right = tds.get(1).cloned().unwrap_or_default();
            let ah = ANCHOR_HREF_RE
                .captures(tr_html)
                .and_then(|c| c.get(1))
                .map(|g| g.as_str().to_string());
            let mut key: Option<&str> = None;
            for (klabel, kval) in PROFILE_MAPPING {
                if left.starts_with(klabel) {
                    key = Some(*kval);
                    break;
                }
            }
            if let Some(k) = key {
                apply_profile_value(k, &right, ah.as_deref());
            }
        }
    } else {
        // Modern layout: reuse precomputed label/value pairs (single-pass extraction).
        for (klabel, kval, klabel_norm) in PROFILE_MAPPING_NORMALIZED.iter() {
            let right = p_pairs.get(*klabel).cloned().or_else(|| {
                p_pairs_norm
                    .get_or_init(|| build_normalized_pairs(&p_pairs))
                    .get(klabel_norm)
                    .cloned()
            });
            if let Some(right) = right {
                apply_profile_value(kval, &right, None);
            }
        }
    }

    // Allocation extraction (highcharts series)
    // Return extraction (Getiri Bilgisi)
    #[cfg(feature = "return_single_pass")]
    {
        capture_return_metrics(&mut res, text, &find_label, true);
    }

    #[cfg(not(feature = "return_single_pass"))]
    {
        capture_return_metrics(&mut res, text, &find_label, false);
        fill_missing_return_metric_fallbacks(&mut res, text);
    }
    // Fallback: if Fon Toplam Değer wasn't captured via table/pairs, try a targeted regex
    if !res.contains_key("fon_toplam_deger_tl_raw")
        && let Some(val) =
            next_p_value_after_any_label(text, &["Fon Toplam Değer (TL)", "Fon Toplam Değer"], true)
        && !val.is_empty()
    {
        res.insert(
            "fon_toplam_deger_tl_raw".to_string(),
            Value::String(decode_html_entities(&val)),
        );
        if let Some(n) = parse_number_tr(Some(&val)) {
            res.insert(
                "fon_toplam_deger_tl".to_string(),
                Value::Number(serde_json::Number::from_f64(n).unwrap()),
            );
        }
    }

    // Fallback: capture 'Kategorisi' / 'Kategori' if not present
    if !res.contains_key("kategori_raw")
        && let Some(val) = next_p_value_after_any_label(text, &["Kategorisi", "Kategori"], false)
        && !val.is_empty()
    {
        res.insert(
            "kategori_raw".to_string(),
            Value::String(decode_html_entities(&val)),
        );
        res.insert(
            "kategori".to_string(),
            Value::String(decode_html_entities(&val)),
        );
    }
    // Fallback: capture 'Pazar Payı' if not present
    if !res.contains_key("pazar_payi_raw_pct")
        && let Some(val) = next_p_value_after_label(text, "Pazar Payı", true)
        && !val.is_empty()
        && let Some(n) = parse_number_tr(Some(&val))
    {
        res.insert(
            "pazar_payi_raw_pct".to_string(),
            Value::Number(serde_json::Number::from_f64(n).unwrap()),
        );
    }

    if !res.contains_key("platform_islem_durumu_raw")
        && let Some(val) =
            next_p_value_after_any_label(text, &["Platform Durumu", "Platform İşlem Durumu"], false)
        && !val.is_empty()
    {
        let decoded = decode_html_entities(&val);
        res.insert(
            "platform_islem_durumu_raw".to_string(),
            Value::String(decoded.clone()),
        );
        res.insert("platform_islem_durumu".to_string(), Value::String(decoded));
    }

    // Fallback: capture 'Son 1 Yıllık Kategori Derecesi' if not present
    if !res.contains_key("son_1yillik_kategori_derecesi_raw")
        && let Some(val) = next_p_value_after_label(text, "Son 1 Yıllık Kategori Derecesi", true)
        && !val.is_empty()
    {
        res.insert(
            "son_1yillik_kategori_derecesi_raw".to_string(),
            Value::String(decode_html_entities(&val)),
        );
        let first_digits: String = val.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(v) = first_digits.parse::<i64>() {
            res.insert(
                "son_1yillik_kategori_derecesi".to_string(),
                Value::Number(v.into()),
            );
        }
    }

    // Fallback: capture 'Yatırımcı Sayısı' if not present
    if !res.contains_key("yatirimci_sayisi_raw")
        && let Some(val) = next_p_value_after_label(text, "Yatırımcı Sayısı", true)
        && !val.is_empty()
    {
        res.insert(
            "yatirimci_sayisi_raw".to_string(),
            Value::String(decode_html_entities(&val)),
        );
        if let Some(n) = parse_i64_from_digits(&val) {
            res.insert("yatirimci_sayisi".to_string(), Value::Number(n.into()));
        }
    }

    let rsc_payloads = if is_rsc {
        Some(extract_next_f_string_payloads(text))
    } else {
        None
    };

    if matches!(doc.kind, DocumentKind::NextJsRsc)
        && let Some(payloads) = rsc_payloads.as_ref()
    {
        enrich_rsc_profile_from_payloads(payloads, &mut res);
    }

    let needs_rsc_fallback = !res.contains_key("fon_kodu") || !res.contains_key("isin_kodu");
    if needs_rsc_fallback
        && let Some(payloads) = rsc_payloads.as_ref()
    {
        let rsc_fast = extract_rsc_fast_fields_from_payloads(payloads);
        for (k, v) in rsc_fast {
            res.entry(k).or_insert(v);
        }
    }

    if matches!(res.get("fon_adi"), Some(Value::Null)) {
        let code = res
            .get("fon_kodu")
            .or_else(|| res.get("fon_kodu_raw"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        if let Some(code) = code
            && !code.trim().is_empty()
        {
            res.insert("fon_adi".to_string(), Value::String(code));
        }
    }

    if !res.contains_key("kap_bilgi_adresi")
        && let Some(link) = extract_kap_link(text)
    {
        res.insert(
            "kap_bilgi_adresi_raw".to_string(),
            Value::String(link.clone()),
        );
        res.insert("kap_bilgi_adresi".to_string(), Value::String(link));
    }

    // Focus expensive chart scans on the script-heavy suffix when available.
    let chart_scope = doc.chart_scope;

    let mut allocation: Vec<Value> = Vec::new();
    let mut blocks = find_series_blocks(chart_scope);
    if blocks.is_empty() {
        for m in SERIES_INIT_RE.find_iter(chart_scope) {
            let br = m.end() - 1;
            if let Some((slice, _end)) = parse_balanced(chart_scope, br, '[', ']') {
                blocks.push(slice.to_owned());
                break;
            }
        }
    }

    'outer_alloc: for b in &blocks {
        if let Some(parsed) = try_parse(b) {
            if let Value::Array(arr) = &parsed {
                // look for pie series
                for obj in arr {
                    if let Value::Object(map) = obj {
                        let name = map
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_lowercase();
                        let typ = map.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        if typ == "pie"
                            || name.contains("varl")
                            || name.contains("dağılım")
                            || name.contains("dagilim")
                        {
                            allocation = normalize_data(map.get("data").unwrap_or(&Value::Null));
                            break 'outer_alloc;
                        }
                    }
                }
                // try candidate heuristic
                let mut candidates: Vec<Value> = Vec::new();
                for obj in arr {
                    if let Value::Object(map) = obj
                        && let Some(data_block) = map.get("data")
                        && let Value::Array(darr) = data_block
                        && darr.len() == 1
                    {
                        let first = &darr[0];
                        let value_opt = match first {
                            Value::Number(_) => Some(first.clone()),
                            Value::Array(a) if !a.is_empty() && a[1].is_number() => {
                                Some(a[1].clone())
                            }
                            Value::Array(a) if !a.is_empty() && a[0].is_number() => {
                                Some(a[0].clone())
                            }
                            _ => None,
                        };
                        if let Some(val) = value_opt {
                            let name = map.get("name").cloned().unwrap_or(Value::Null);
                            candidates.push(json!({"name": name, "value": val}));
                        }
                    }
                }
                if candidates.len() >= 3 && (candidates.len() as f64) >= (arr.len() as f64 / 2.0) {
                    allocation = candidates;
                    break;
                }
            } else if let Value::Object(map) = &parsed {
                let name = map
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_lowercase();
                let typ = map.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if typ == "pie"
                    || name.contains("varl")
                    || name.contains("dağılım")
                    || name.contains("dagilim")
                {
                    allocation = normalize_data(map.get("data").unwrap_or(&Value::Null));
                    break;
                }
            }
        }
    }

    // Allocation fallback: try to parse highcharts config objects
    if allocation.is_empty()
        && let Some(hc_conf) = (|| {
            let candidates = ["new Highcharts.Chart", "Highcharts.chart"];
            for kw in &candidates {
                let mut idx = 0usize;
                while let Some(pos) = chart_scope[idx..].find(kw) {
                    let pos = idx + pos;
                    let par = chart_scope[pos..].find('(');
                    if let Some(par) = par {
                        let par = pos + par;
                        if let Some((slice, _end)) = parse_balanced(chart_scope, par, '(', ')') {
                            let args_text = &slice[1..slice.len() - 1];
                            let args = split_top_level_args(args_text);
                            for astr in args {
                                if astr.starts_with('{') {
                                    if let Some(v) = try_parse(astr) {
                                        return Some(v);
                                    }
                                } else {
                                    // try to parse series: [...] inside
                                    if let Some(m2) = SERIES_INIT_RE.find(astr) {
                                        let br = m2.end() - 1;
                                        if let Some((slice2, _)) =
                                            parse_balanced(astr, br, '[', ']')
                                            && let Some(v2) = try_parse(slice2)
                                        {
                                            return Some(json!({"series": v2}));
                                        }
                                    }
                                }
                            }
                            idx = pos + 1;
                            continue;
                        }
                    }
                    break;
                }
            }
            None
        })()
    {
        let alloc_try = extract_allocation_from_highcharts_config(&hc_conf);
        if !alloc_try.is_empty() {
            allocation = alloc_try;
        }
    }

    // Allocation fallback: try parsing HTML tables (modern TEFAS layout)
    if allocation.is_empty() {
        let tbl_alloc = extract_allocation_from_table(text);
        if !tbl_alloc.is_empty() {
            allocation = tbl_alloc;
        }
    }

    // Price history extraction
    let mut history: Vec<Value> = Vec::new();
    let mut dates: Vec<Value> = Vec::new();
    let mut price_data: Vec<Value> = Vec::new();
    let x_blocks = find_xaxis_blocks(chart_scope);
    for b in &x_blocks {
        if let Some(parsed) = try_parse(b)
            && let Value::Array(arr) = &parsed
            && !arr.is_empty()
            && let Some(Value::Object(first)) = arr.first()
            && let Some(categories) = first.get("categories")
            && categories.is_array()
        {
            dates = categories.as_array().unwrap().clone();
            break;
        }
    }
    for b in &blocks {
        if let Some(parsed) = try_parse(b)
            && let Value::Array(arr) = parsed
        {
            for obj in arr {
                if let Value::Object(map) = obj
                    && map.get("name").and_then(|v| v.as_str()) == Some("Fiyat")
                    && let Some(data) = map.get("data")
                    && let Value::Array(darr) = data
                {
                    price_data = darr.clone();
                    break;
                }
            }
            if !price_data.is_empty() {
                break;
            }
        }
    }
    if !dates.is_empty() && !price_data.is_empty() {
        for (d, p) in dates.iter().zip(price_data.iter()) {
            history.push(json!({"date": d, "price": p}));
        }
    }

    // flat copy
    let mut flat_res = Map::new();
    for (k, v) in res.iter() {
        flat_res.insert(k.clone(), v.clone());
    }
    flat_res.insert("allocation".to_string(), Value::Array(allocation.clone()));
    flat_res.insert("history".to_string(), Value::Array(history.clone()));

    // grouped output
    let profile_base_keys = vec![
        "fon_kodu",
        "isin_kodu",
        "platform_islem_durumu",
        "islem_baslangic_saati",
        "son_islem_saati",
        "fon_alis_valoru",
        "fon_toplam_deger_tl",
        "kategori",
        "pazar_payi_raw_pct",
        "son_1yillik_kategori_derecesi",
        "yatirimci_sayisi",
        "fon_satis_valoru",
        "min_alis_islem_miktari",
        "min_satis_islem_miktari",
        "max_alis_islem_miktari",
        "max_satis_islem_miktari",
        "giris_komisyonu",
        "cikis_komisyonu",
        "fon_faiz_icerigi",
        "fon_risk_degeri",
        "kap_bilgi_adresi",
    ];
    let mut fund_profile = Map::new();
    for base in profile_base_keys {
        let raw_key = format!("{}_raw", base);
        if let Some(v) = res.remove(&raw_key) {
            fund_profile.insert(raw_key, v);
        }
        if let Some(v) = res.remove(base) {
            fund_profile.insert(base.to_string(), v);
        }
    }

    // Normalize and add boolean flag whether the fund is traded on TEFAS
    let platform_raw_decoded = match fund_profile.get("platform_islem_durumu_raw") {
        Some(Value::String(raw)) => decode_html_entities(raw),
        _ => String::new(),
    };
    let s = platform_raw_decoded.to_lowercase();
    let is_trading = s.contains("işlem görü")
        || s.contains("islem gor")
        || s.contains("işlem gör")
        || s.contains("görüyor");
    // overwrite normalized raw (empty string if absent) and always insert boolean
    fund_profile.insert(
        "platform_islem_durumu_raw".to_string(),
        Value::String(platform_raw_decoded),
    );
    fund_profile.insert(
        "platform_islem_goruyor".to_string(),
        Value::Bool(is_trading),
    );

    let return_keys = vec![
        "son_1ay_getiri_raw",
        "son_1ay_getiri_pct",
        "son_3ay_getiri_raw",
        "son_3ay_getiri_pct",
        "son_6ay_getiri_raw",
        "son_6ay_getiri_pct",
        "son_1yil_getiri_raw",
        "son_1yil_getiri_pct",
        "son_3yil_getiri_raw",
        "son_3yil_getiri_pct",
        "son_5yil_getiri_raw",
        "son_5yil_getiri_pct",
    ];
    let mut returns = Map::new();
    for k in return_keys {
        if let Some(v) = res.remove(k) {
            returns.insert(k.to_string(), v);
        }
    }

    let mut indicator = Map::new();
    for (k, v) in res.iter() {
        indicator.insert(k.clone(), v.clone());
    }
    indicator.insert("allocation".to_string(), Value::Array(allocation));

    let grouped = json!({"indicator": Value::Object(indicator), "profile": Value::Object(fund_profile), "return": Value::Object(returns), "history": Value::Array(history)});

    (grouped, Value::Object(flat_res))
}

// Try to extract allocation from an HTML table (modern TEFAS layout)
fn extract_allocation_from_table(text: &str) -> Vec<Value> {
    let out: Vec<Value> = Vec::new();

    // prefer table that follows the 'Fon Varlık Dağılımı' heading
    if let Some(m) = H3_VARLIK_RE.find(text)
        && let Some(cap) = TABLE_RE.captures(&text[m.end()..])
    {
        let tbl = cap.get(1).map(|g| g.as_str()).unwrap_or("");
        let parsed = parse_allocation_table(tbl);
        if !parsed.is_empty() {
            return parsed;
        }
    }

    // otherwise scan all tables and pick the first one that looks like allocation
    for cap in TABLE_RE.captures_iter(text) {
        let tbl = cap.get(1).map(|g| g.as_str()).unwrap_or("");
        let lower = tbl.to_lowercase();
        if lower.contains("varl") || lower.contains("oran") || lower.contains("varlik") {
            let parsed = parse_allocation_table(tbl);
            if parsed.len() >= 2 {
                return parsed;
            }
        }
    }

    out
}

fn parse_allocation_table(tbl_inner: &str) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();

    for tr in TR_RE.captures_iter(tbl_inner) {
        let tr_html = tr.get(1).map(|g| g.as_str()).unwrap_or("");
        let mut cells: Vec<String> = Vec::new();
        for c in CELL_RE.captures_iter(tr_html) {
            let raw = c.get(1).map(|g| g.as_str()).unwrap_or_default();
            let no_tags = strip_html_tags(raw);
            let s = clean_whitespace(Some(&no_tags));
            cells.push(s);
        }
        if cells.is_empty() {
            continue;
        }
        // header row -> skip if header-like (contains 'Varlık' or 'Oran')
        let first_lc = cells.first().map(|s| s.to_lowercase()).unwrap_or_default();
        let second_lc = cells.get(1).map(|s| s.to_lowercase()).unwrap_or_default();
        if first_lc.contains("varl") || second_lc.contains("oran") {
            continue;
        }

        // prefer second column as value, otherwise find any parsable number in row
        let mut val_opt: Option<f64> = None;
        if let Some(vs) = cells.get(1) {
            val_opt = parse_number_tr(Some(vs));
        }
        if val_opt.is_none() {
            for cell in &cells {
                if let Some(vv) = parse_number_tr(Some(cell)) {
                    val_opt = Some(vv);
                    break;
                }
            }
        }
        if let Some(v) = val_opt {
            let name = cells
                .first()
                .map(|s| decode_html_entities(s))
                .unwrap_or_default();
            out.push(json!({"name": name, "value": v}));
        }
    }

    out
}

#[cfg(test)]
mod tests;
