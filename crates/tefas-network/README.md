# tefas-network

Geriye dönük uyumluluk adaptör crate'i.

**Paket adı:** `tefas-network` &nbsp;·&nbsp; **Dizin:** `crates/tefas-network`

---

## Sorumluluk

Bu crate artık implementasyon içermez; shared workspace altındaki `http-client`
crate'inin public API'sini re-export eder:

- `pub use http_client::*;`

Bu sayede mevcut `tefas-network` importları kırılmadan çalışmaya devam eder.

---

## Feature Flags

| Flag | Default | Açıklama |
|------|---------|----------|
| `native-tls` | ✅ | shared `http-client/native-tls` geçişi |
| `hyper-backend` | ✅ | shared `http-client/hyper-backend` geçişi |
| `wreq-backend` | ✅ | shared `http-client/wreq-backend` geçişi |

---

## Örnek

```rust
use tefas_network::NetworkClient;
```

---

## İlgili

- [../../docs/architecture.md](../../docs/architecture.md) — shared + tefas sınırları
- [../../shared/crates/http-client/README.md](../../shared/crates/http-client/README.md) — taşınan ağ/TLS backend implementasyonu
