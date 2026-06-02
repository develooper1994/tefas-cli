use super::*;

#[test]
fn extract_next_f_string_payloads_decodes_json_literals() {
    let html = r#"
    <script>
    self.__next_f.push([1,"{\"fonKodu\":\"AC5\",\"isinKodu\":\"TRXYZ\"}"]);
    self.__next_f.push([2,"{\"kapLink\":\"https://kap.org.tr/test\"}"]);
    </script>
    "#;

    let payloads = extract_next_f_string_payloads(html);
    assert_eq!(payloads.len(), 2);
    assert!(payloads[0].contains("\"fonKodu\":\"AC5\""));
    assert!(payloads[1].contains("\"kapLink\":"));
}

#[test]
fn extract_rsc_fast_fields_collects_known_profile_fields() {
    let html = r#"
    <script>
    self.__next_f.push([1,"{\"fonKodu\":\"AC5\",\"isinKodu\":\"TRYAT123\"}"]);
    self.__next_f.push([2,"{\"kapLink\":\"https://kap.org.tr/tr/Bildirim/1\"}"]);
    </script>
    "#;

    let payloads = extract_next_f_string_payloads(html);
    let out = extract_rsc_fast_fields_from_payloads(&payloads);
    assert_eq!(out.get("fon_kodu").and_then(|v| v.as_str()), Some("AC5"));
    assert_eq!(
        out.get("isin_kodu").and_then(|v| v.as_str()),
        Some("TRYAT123")
    );
    assert_eq!(
        out.get("kap_bilgi_adresi").and_then(|v| v.as_str()),
        Some("https://kap.org.tr/tr/Bildirim/1")
    );
}

#[test]
fn extract_rsc_fast_fields_collects_numeric_fields() {
    let html = r#"
    <script>
    self.__next_f.push([1,"{\"fonToplamDeger\":\"1.234.567,89\",\"pazarPayi\":\"2,50\",\"fonRiskDegeri\":\"4 / 7\"}"]);
    </script>
    "#;

    let payloads = extract_next_f_string_payloads(html);
    let out = extract_rsc_fast_fields_from_payloads(&payloads);
    assert_eq!(
        out.get("fon_toplam_deger_tl")
            .and_then(|v| v.as_f64())
            .map(|v| (v * 100.0).round() / 100.0),
        Some(1234567.89)
    );
    assert_eq!(
        out.get("pazar_payi_raw_pct").and_then(|v| v.as_f64()),
        Some(2.5)
    );
    assert_eq!(out.get("fon_risk_degeri").and_then(|v| v.as_i64()), Some(4));
}

#[test]
fn parse_balanced_handles_multibyte_prefix_and_nested_strings() {
    let html =
        "Önek içerik self.__next_f.push([1,{\"metin\":\"ç) [kal]\",\"nested\":[{\"x\":1}]}]); son";
    let open = html.find('(').unwrap();

    let (slice, end) = parse_balanced(html, open, '(', ')').unwrap();

    assert_eq!(
        slice,
        "([1,{\"metin\":\"ç) [kal]\",\"nested\":[{\"x\":1}]}])"
    );
    assert_eq!(html.as_bytes()[end], b')');
    assert!(html.is_char_boundary(end));
}
