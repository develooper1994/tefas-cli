# Architecture

## Crate Dependency Graph

```
tefas-cli  (binary)
  ├── tefas-usecase  (use-case orchestration/planning)
  │     └── tefas-domain
  ├── tefas-domain   (domain models + error taxonomy)
  ├── tefas-config   (AppConfig + TEFAS defaults)
  ├── tefas-api      (Operation/OperationOld descriptors)
  ├── tefas-network  (compat adapter -> shared/http-client)
  ├── tefas-parser   (HTML parser: single backend)
  ├── tefas-media    (TEFAS getLogo adapter -> shared/image-util)
  └── tefas-tools    (dep installer, dev utilities)

shared workspace
  ├── http-config    (generic HttpBackend/TlsBackend/TlsConfig/RetryConfig)
  ├── http-client    (generic HTTP execution: reqwest / hyper / impcurl / wreq)
  └── image-util     (generic base64 image conversion)

etl root
  └── data-collection-cli (root orchestration + child CLI delegation + catalog discovery)
```

Crate adları ile dizin adları birebir eşleşmez. Aşağıdaki tablo canonical mapping'i gösterir:

| Dizin (`crates/`) | Paket adı (`name =`) | Sorumluluk |
|---|---|---|
| `tefas-core` | `tefas-config` | TEFAS odaklı config: `AppConfig`, `AuthConfig`, TEFAS default URL/UA/referer |
| `tefas-domain` | `tefas-domain` | Domain çekirdeği: `FundProfile`, `FundReturns`, `DomainError`, `QueryOperationName` |
| `tefas-usecase` | `tefas-usecase` | Use-case planlama katmanı: query/fundpage/fetch batch policy ve orkestrasyon planları |
| `tefas-funds` | `tefas-api` | Takasbank/TEFAS operation descriptor: endpoint, referer, default payload |
| `tefas-network` | `tefas-network` | Geriye dönük uyumluluk adaptörü (`pub use http_client::*`) |
| `tefas-page` | `tefas-parser` | Fund page HTML parser: tek backend parse yolu |
| `tefas-cli` | `tefas-cli` | CLI entry point (clap); tüm alt sistemleri birleştirir |
| `tefas-media` | `tefas-media` | TEFAS `getLogo` alan eşleme + shared `image-util` delegasyonu |
| `tefas-tools` | `tefas-tools` | Geliştirici araçları: bağımlılık kurulum fallback mantığı |

---

## Design Principles

### 0 — Domain + Use-Case ayrımı
Domain modelleri (`tefas-domain`) IO bağımsız tutulur. Komutlara özgü concurrency/pipeline kararları `tefas-usecase` içinde toplanır.

Bu sayede:

- CLI yalnızca adapter görevi görür (arg parse + output render).
- Network/parser değişimleri use-case sınırlarını bozmaz.
- Politikalar (örn. query batch concurrency) test edilebilir hale gelir.

### 1 — Ağ katmanı domain'den ayrı
`tefas-api` yalnızca **ne** isteneceğini tanımlar (endpoint, payload şeması).
**Nasıl** gönderileceği shared `http-client` crate'inin sorumluluğundadır (`tefas-network` yalnızca adaptördür).  
Bu sayede network stack'i değiştirmek domain kodunu etkilemez.

### 2 — Tek Backend Parser (fund_page)
`tefas-parser::fund_page::parse_document` tek parse yolunu (`parse_html_text`) kullanır.

Bu sadeleştirme ile:

- Backend seçim/dispatch maliyeti kaldırılır.
- Feature-flag kaynaklı davranış ayrışmaları azaltılır.
- Profil çıktılarında hotspot analizi daha deterministik hale gelir.

`parse_document` her zaman normalize edilmiş `(grouped, meta)` çıktısı döndürür.

### 3 — Config tipik olarak CLI'dan oluşturulur
`AppConfig` CLI argümanlarından inşa edilir ve `NetworkClient`'e geçirilir.  
Kütüphane kullanıcıları `AppConfig::with_defaults()` ile doğrudan inşa edebilir.

### 4 — Fuzz workspace izolasyonu
Fuzz hedefleri artık ayrı bir Cargo projesi değildir; doğrudan `xtask` paketinin feature-gated binary hedefleri olarak çalışır.  
Çalıştırma örneği: `cargo run -p xtask --features fuzzing --bin fuzz_parse_document -- -runs=1000`.  
Otomatik smoke fuzz için: `cargo xtask tefas fuzz --dry-run`

### 5 — Root CLI orchestration
`data-collection-cli` ust katmanda iki sorumlulugu toplar:

- katalog ve plan kesfi (`layers`, `pipelines`, `products`, `analytics`)
- alt CLIlara delegasyon (`data-collection-cli tefas ...`, `data-collection-cli kap ...`)

Bu ayrim sayesinde her kaynak kendi CLI sinirlarini korur, ama smoke test ve platform katmani icin tek bir giris noktasi da olur.

### 6 — Veri urunu once, ekran sonra
TEFAS verisi once veri urunleri halinde normalize edilmelidir:

- `fund_snapshot`
- `return_panel`
- `cost_and_management`
- `breadth_and_flow`

Bu veri urunleri daha sonra backtest, risk tarama, turetilmis metrik ve trend tespiti gibi analitiklerin girdisi olur. Uygulama fazlari icin [../../../docs/data_collection_plan.md](../../../docs/data_collection_plan.md) dosyasina bak.

---

## Feature Flags

### `http-client` (shared)
| Flag | Etkinleştirdiği |
|------|----------------|
| `native-tls` (default) | reqwest + hyper için native-tls-vendored + hyper-tls |
| `hyper-backend` (default) | hyper + hyper-util + rustls backend desteği |
| `wreq-backend` (default) | wreq backend desteği |

### `tefas-parser`
Şu anda parser tarafında backend seçimi için özel feature flag kullanılmıyor.

---

## Test Yapısı

| Konum | Tür | Kapsam |
|-------|-----|--------|
| `crates/*/src/**` `#[cfg(test)]` | unit | izole fonksiyon mantığı |
| `crates/parser/tests/compare_datasets.rs` | integration | parser parity + FundPage API |
| `crates/parser/benches/parse_document_bench.rs` | criterion bench | parse throughput |
| `xtask/src/bin/fuzz_*.rs` | libFuzzer | parser + network crash bulma |
| `cargo xtask tefas fuzz --dry-run` | smoke fuzz | tüm operasyon × backend matris testi |

```bash
# Tüm testleri çalıştır
CARGO_HTTP_CAINFO=/etc/ssl/certs/ca-certificates.crt cargo test --workspace

# Bench (HTML raporu üretir)
CARGO_HTTP_CAINFO=/etc/ssl/certs/ca-certificates.crt \
  cargo bench -p tefas-parser

# Smoke fuzz (gerçek ağ gerekmez)
cargo xtask tefas fuzz --dry-run
```

---

## Açık Kalan İşler

Detaylar için: [../TODO.md](../TODO.md)

- **WAF bypass** — kaynak edinme bekliyor (#1, #15–#20)
- **Parser fixture parity** — gerçek fon-analiz HTML fixture'ı bekliyor (#3, #6)
- **Low-level HTTP** — araştırma tamamlandı, opsiyonel uygulama (#14 → [operations/low_level_http.md](operations/low_level_http.md))

## Parser Tasarım Notları

Fund page parser için son iki tasarım girdisi ayrı belgelerde tutuluyor:

- [parser-hotspot-plan.md](parser-hotspot-plan.md) — regex/string hotspot'larının hangi sırayla ve hangi guardrail'lerle ele alınacağı
- [parser-rsc-legacy-refactor.md](parser-rsc-legacy-refactor.md) — `parse_html_text` içindeki RSC/legacy ayrımının önerilen hedef yapısı
