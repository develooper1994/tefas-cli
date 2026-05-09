# tefas-config

Tüm `tefas-*` crate'lerinde paylaşılan runtime konfigürasyon tipleri.

**Paket adı:** `tefas-config` &nbsp;·&nbsp; **Dizin:** `crates/core`

---

## Sorumluluk

- `AppConfig`: CLI'dan oluşturulan birleşik konfigürasyon nesnesi  
- `HttpBackend`: hangi network stack'in kullanılacağı (`Reqwest` / `Hyper` / `Impcurl`)  
- `TlsConfig` + `TlsBackend`: TLS uygulaması ve bağlantı timeout'u  
- `RetryConfig`: yeniden deneme sayısı ve backoff gecikmesi  
- `AuthConfig`: token, cookie jar, User-Agent, Referer  

Bu crate kasıtlı olarak **ağ bağımlılığı içermez**. Sadece `serde` bağımlılığına sahiptir.

---

## Örnek

```rust
use tefas_config::{AppConfig, HttpBackend, TlsBackend, TlsConfig};

let cfg = AppConfig {
    backend: HttpBackend::Hyper,
    tls: TlsConfig { backend: TlsBackend::NativeTls, timeout_ms: Some(5_000) },
    ..AppConfig::with_defaults()
};
```

---

## İlgili

- [docs/architecture.md](../../docs/architecture.md) — crate bağımlılık grafiği
- `tefas-network` — bu config'i tüketen HTTP client
