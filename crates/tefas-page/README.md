# tefas-parser

TEFAS fon analiz HTML sayfası ayrıştırıcı.

**Paket adı:** `tefas-parser` &nbsp;·&nbsp; **Dizin:** `crates/tefas-page`

---

## Sorumluluk

- `TefasPage` trait: `url()`, `validate()`, `parse()` arayüzü
- `FundPage`: `TefasPage` implementasyonu (fund analiz sayfası)
- `parse_document(html: &str) -> (Value, Value)`: tek backend parse ve normalize çıktı

### Parser Akışı

`parse_document` tek parse yolunu (`parse_html_text`) çalıştırır ve çıktıyı normalize eder.
Amaç: daha az kontrol akışı karmaşası ve daha tutarlı performans profili.

### WAF Tespiti

`contains_failureconfig(html: &str) -> bool` — şüpheli WAF/challenge marker varlığını kontrol eder. `tefas-cli` bu fonksiyonu preflight sonucunu doğrulamak için kullanır.

---

## Bench

```bash
CARGO_HTTP_CAINFO=/etc/ssl/certs/ca-certificates.crt \
  cargo bench -p tefas-parser
```

Criterion HTML raporu: `target/criterion/`

`AC5` / `AFT` / `TLY` gerçek fon sayfası fixture'ları `../../datasets/tefas/fundpage` altındadır ve benchmarkta `include_str!` ile gömülür.

Samply profili:

```bash
CARGO_PROFILE_BENCH_DEBUG=true CARGO_PROFILE_BENCH_STRIP=false \
  samply record -s -o /tmp/tefas-parser-final-profile.json.gz \
  cargo bench -p tefas-parser --bench parse_document_bench -- --profile-time 5 parse_document_batch
```

---

## Test

```bash
cargo test -p tefas-parser
```

Integration testler: `tests/compare_datasets.rs` — parser parity + `FundPage` API

---

## Feature Flags

Parser backend seçimi için aktif feature flag kullanılmaz.

---

## İlgili

- [docs/architecture.md](../../docs/architecture.md) — parser mimarisi açıklaması
- `tefas-media` — parse çıktısındaki logo verisi için görüntü dönüşümü
