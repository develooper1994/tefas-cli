# http-client

Generic async HTTP client crate.

## Kapsam

- Backend secimi: wreq / reqwest / hyper / impcurl
- TLS secimi: rustls / native-tls / insecure (backend uyumuna gore)
- Header/cookie hazirlama
- JSON/text request helperlari
- Retry ve timeout akislari

Bu crate, onceki tefas-network implementasyonunun shared katmana tasinmis halidir.

## Features

- `hyper-backend`
- `wreq-backend`
- `native-tls`
- `tefas-compat` (default): `tefas_config::AppConfig` ile backward-compatible `NetworkClient::new(&AppConfig)` giris noktasi

KAP gibi yeni projelerde TEFAS bagimliligini kapatmak icin:

```toml
http-client = { path = "crates/http-client", default-features = false, features = ["hyper-backend", "wreq-backend"] }
```
- `native-tls`

## Build

```bash
cd shared
cargo check -p http-client
```
