# Operations

Operation tabanlı CLI kullanımı için belge merkezi.

## Belgeler

- `cargo xtask tefas probe [FUND_CODE|URL]` — TEFAS fetch diagnostics (cli / curl / curl-impersonate)
- `cargo xtask tefas bench [--host ALIAS --repo PATH --cargo-profile NAME --filter EXPR --warmup N --measure N --samply|--no-samply --flamegraph]` — Remote perf pipeline
- `cargo xtask tefas profile [same options as bench]` — Remote perf pipeline (samply zorunlu açık)
- `cargo xtask tefas perf-remote [same options as bench]` — `bench` ile aynı remote pipeline giriş noktası
- `cargo xtask tefas samply-remote [--host ALIAS --repo PATH --cargo-profile NAME --workload parse|fundpage|query|fetch --output FILE]` — Remote save-only samply trace akışı

Not: Geriye dönük uyumluluk için `cargo xtask probe|bench|profile|perf-remote|samply-remote` kısa yolları halen çalışır.

## Hızlı Referans

Operasyon tanımlayıcıları için → [`tefas-api` (crates/tefas-funds)](../../crates/tefas-funds/README.md)  
CLI flag detayları için → [`tefas-cli` (crates/tefas-cli)](../../crates/tefas-cli/README.md)

## Yardım Komutları

```bash
cargo run -q -p tefas-cli -- --help
cargo run -q -p tefas-cli -- fundpage --help
cargo run -q -p tefas-cli -- query --help
cargo run -q -p tefas-cli -- fetch --help
```

## Temel Kural

Operation çağrılarında eski `--operation` / `--operationOld` flagleri kullanılmaz. Yeni sözdizimi `query <OP>` ve legacy uçlar için `query --old <OP>` şeklindedir.
