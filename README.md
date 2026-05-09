# tefas

TEFAS / Takasbank fon veri araçları için Rust workspace'i.

CLI, Takasbank JSON API'sine karşı ham HTTP istekleri gönderir; scraping yoktur.
Sonuçlar JSON veya okunabilir text olarak çıktılanır.

---

## Hızlı Başlangıç

```bash
# Fon sayfalarını paralel çek ve parse et
cargo run -q -p tefas-cli -- fundpage AC5 TLY AFT

# Ham HTML'leri bir klasöre yedekle
cargo run -q -p tefas-cli -- fundpage AC5 TLY --save-html ./backups

# Takasbank API operasyonlarını listele
cargo run -q -p tefas-cli -- query --list

# Belirli bir operasyonun giriş alanlarını gör
cargo run -q -p tefas-cli -- query --info fonBilgiGetir

# Birden fazla operasyon (sonuçlar tek JSON'da birleşir)
cargo run -q -p tefas-cli -- query fonBilgiGetir getBanners

# Payload alanı override
cargo run -q -p tefas-cli -- query fonBilgiGetir --set fonKodu=AC5

# Birden fazla URL'yi paralel çek
cargo run -q -p tefas-cli -- fetch https://www.tefas.gov.tr https://www.takasbank.com.tr --output ./html_dump

# Yerel HTML dosyalarını parse et
cargo run -q -p tefas-cli -- parse tefas_fetched.html other_fund.html

# Logo PNG olarak kaydet
cargo run -q -p tefas-cli -- logo AC5 TLY --format png
```

Daha fazla örnek: [docs/operations/quickstart.md](docs/operations/quickstart.md)

---

## Crate Yapısı
...
```
tefas-cli  ──►  tefas-lib (crate name: tefas)
       ├── tefas-usecase ──► tefas-domain
       ├── tefas-config
       ├── tefas-api
       ├── tefas-network (compat adapter) ──► shared/http-client
       ├── tefas-parser
       └── tefas-media (TEFAS adapter) ──► shared/image-util
     ──►  tefas-tools

shared workspace:
  - http-config
  - http-client
  - image-util
```

| Dizin (`crates/`) | Paket | Sorumluluk |
|---|---|---|
| [tefas-lib](crates/tefas-lib/README.md) | `tefas-lib` (`lib name: tefas`) | CLI facade + executor katmanı (`run_fundpage_batch`, `run_query_batch`, `run_fetch_batch`) |
| [tefas-domain](crates/tefas-domain/README.md) | `tefas-domain` | IO bağımsız domain modelleri ve hata sınıfları |
| [tefas-usecase](crates/tefas-usecase/README.md) | `tefas-usecase` | Batch planlama/policy katmanı (query/fundpage/fetch concurrency) |
| [tefas-core](crates/tefas-core/README.md) | `tefas-config` | `AppConfig`, `AuthConfig` ve TEFAS default URL/UA/referer |
| [tefas-funds](crates/tefas-funds/README.md) | `tefas-api` | `Operation` enum (32 varyant), `OperationSpec` (endpoint + referer) |
| [tefas-network](crates/tefas-network/README.md) | `tefas-network` | Geriye dönük uyumluluk adaptörü (`pub use http_client::*`) |
| [tefas-page](crates/tefas-page/README.md) | `tefas-parser` | HTML parser: lol→html5ever→tl→regex fallback zinciri |
| [tefas-cli](crates/tefas-cli/README.md) | `tefas-cli` | CLI binary (clap), tüm crate'leri koordine eder |
| [tefas-ffi](crates/tefas-ffi/README.md) | `tefas-ffi` | FFI giriş noktası (CLI default build zincirinde zorunlu değil) |
| [tefas-media](crates/tefas-media/README.md) | `tefas-media` | TEFAS `getLogo` alan eşleme (dönüşüm shared `image-util` ile) |
| [tefas-tools](crates/tefas-tools/README.md) | `tefas-tools` | `install_with_fallback`: apt/dnf/yum/pacman/apk |

---

## Dokümantasyon

| Belge | İçerik |
|-------|--------|
| [docs/README.md](docs/README.md) | Dokümantasyon dizini |
| [docs/architecture.md](docs/architecture.md) | Crate bağımlılık grafiği, tasarım kararları |
| [docs/operations/quickstart.md](docs/operations/quickstart.md) | Hızlı kullanım örnekleri |
| [docs/operations/reference.md](docs/operations/reference.md) | Tüm flag ve davranış referansı |
| [docs/operations/low_level_http.md](docs/operations/low_level_http.md) | Low-level HTTP araştırması |
| [docs/api/INPUT_OUTPUT_FIELDS.md](docs/api/INPUT_OUTPUT_FIELDS.md) | API alan referansı |
| [shared/README.md](shared/README.md) | Ortak crate'ler (http-config/http-client/image-util) |
| [shared/crates/http-client/README.md](shared/crates/http-client/README.md) | Taşınan ağ/TLS backend katmanı |
| [shared/crates/http-config/README.md](shared/crates/http-config/README.md) | Taşınan HTTP config katmanı |

---

## Geliştirme

### Build & Test

```bash
# Hızlı yol (default-members = tefas-cli): yalnız CLI + transitif bağımlılıkları
CARGO_HTTP_CAINFO=/etc/ssl/certs/ca-certificates.crt cargo check
CARGO_HTTP_CAINFO=/etc/ssl/certs/ca-certificates.crt cargo test

# Tüm workspace (ffi/xtask/diger crate'ler dahil)
CARGO_HTTP_CAINFO=/etc/ssl/certs/ca-certificates.crt cargo check --workspace
CARGO_HTTP_CAINFO=/etc/ssl/certs/ca-certificates.crt cargo test --workspace

# Criterion benchmark (tefas-parser)
CARGO_HTTP_CAINFO=/etc/ssl/certs/ca-certificates.crt cargo bench -p tefas-parser
```

### TLS Profilleri

Runtime varsayılanı `wreq` backend + `rustls` TLS seçeneğidir; default feature seti `hyper` ve `wreq` backendlerini derler:

```bash
cargo check --workspace   # default features
```

Hyper + NativeTLS profili sistem TLS kütüphanelerini gerektirir:

```bash
cargo run -q -p tefas-cli --features native-tls -- \
  --backend hyper --tls nativetls query fonBilgiGetir
```

> `wreq` backend kendi BoringSSL parmak izini kullanır; `--tls` seçimi bu backend için etkisizdir.
> `hyper` backend `--tls insecure` seçeneğini desteklemez; bunun için `--backend reqwest --tls insecure` kullanın.

### İmpersonation (curl-impersonate)

Farklı OS/cihaz tarayıcısı kimliklerini taklit etmek için persona preset'leri:

```bash
cargo run -q -p tefas-cli -- \
  --backend impcurl --persona ios query fonBilgiGetir
```

Manuel hedef:

```bash
cargo run -q -p tefas-cli -- \
  --backend impcurl --impersonate chrome136 query fonBilgiGetir
```

### Fuzz

```bash
cargo run -p xtask --features fuzzing --bin fuzz_parse_document -- -runs=1000
```

Smoke fuzz (ağ gerektirmez):

```bash
cargo xtask tefas fuzz --dry-run
```

---

## Yardım

```bash
cargo run -q -p tefas-cli -- --help
cargo run -q -p tefas-cli -- fundpage --help
cargo run -q -p tefas-cli -- query --help
cargo run -q -p tefas-cli -- fetch --help
cargo run -q -p tefas-cli -- parse --help
cargo run -q -p tefas-cli -- logo --help
```

TODO ve handoff notlari: [TODO.md](TODO.md)


## TODO

### Simdiki Durum
- Parity ve parser testleri: yesil

### Sonraki Isler (Perf-Only)
- [ ] Her degisiklikte zorunlu gate: `cargo test -p tefas-parser` + `compare_datasets`.

### Handoff Komutlari

```bash
# 1) Perf-only otomatik kosu
cargo xtask tefas bench \
	--measure 1 \
	--warmup 1 \
	--filter 'parse_document/AC5'

# 2) Son kosunun perf raporunu ac
latest="$(ls -1 target/perf-artifacts | tail -n 1)"
sed -n '1,200p' "target/perf-artifacts/$latest/perf_report.txt"

# 3) Tum dataset sayfalari benchmark/perf ozeti
cat "target/perf-artifacts/$latest/bench_summary.txt"

# 4) Gerekirse samply ac
cargo xtask tefas bench --samply
```

### Notlar
Operasyonel profiling notlari: [docs/operations/profiling.md](docs/operations/profiling.md)
- Perf raporunda ilk hedef regex backtracking maliyeti ve HTML etiket temizleme/parse gecisleri.