# Low-Level HTTP Control: Research Notes

**İş #14** — Takip: `tefas/TODO.md`

---

## Özet

Mevcut network katmanı reqwest (0.12, Rustls/NativeTLS/BoringTLS), Hyper (1.x) ve
impcurl (rquest tabanlı) backend'lerini desteklemektedir. Bu belge, doğrudan
libcurl bağlamaları veya raw socket erişiminin ne kazandırabileceğini değerlendirir.

---

## Mevcut Durum

| Backend | Kütüphane | TLS Seçenekleri | HTTP/2 | Parmak İzi Kontrolü |
|---------|-----------|-----------------|--------|---------------------|
| `reqwest` | reqwest 0.12 | rustls / native-tls / boring | ✅ | ❌ |
| `hyper` | hyper 1.x | rustls / native-tls | ✅ | ❌ |
| `impcurl` | rquest (curl impersonation) | rustls / native-tls | ✅ | ✅ (JA3/JA4/HTTP/2 fingerprint) |

---

## Seçenek 1: `curl` crate (libcurl Rust bağlaması)

**Crate**: [`curl`](https://crates.io/crates/curl) (libcurl-sys üzerine güvenli sarmalayıcı)

### Avantajlar
- **Tam libcurl erişimi**: transfer seçenekleri, cookie jar, verbose debug, multi handle
- **CURLOPT_RESOLVE** ile SNI override: WAF bypass için belirli IP'e yönlendirme
- **CURLOPT_INTERFACE**: belirli ağ arayüzü üzerinden gönderim
- **Pause / resume**: yavaş yükleme simülasyonu (rate limiting emülasyonu için)
- **CURLOPT_PINNEDPUBLICKEY**: public key pinning
- **Multi interface**: async olmadan tek iş parçacığında çoklu eşzamanlı transfer

### Dezavantajlar
- `libcurl` sistem bağımlılığı (statik derleme için `curl-src` feature gerekir)
- `async` native yok — Tokio ile entegrasyon için `spawn_blocking` gerekir
- Mevcut `BackendClient` trait'ine uyarlamak için sarmalayıcı katman yazılmalı
- Docker/CI ortamlarında `libcurl-dev` paketi gerektirir

### Sonuç
Orta düzey kontrol için uygun. Özellikle CURLOPT_RESOLVE ve INTERFACE seçenekleri
test ortamlarında kullanışlıdır. Üretim WAF bypass için yeterli değil.

---

## Seçenek 2: Raw TCP/TLS Sockets (tokio + rustls)

**Yaklaşım**: `tokio::net::TcpStream` → `rustls::ClientConnection` → HTTP/1.1 manuel

### Avantajlar
- **Tam kontrol**: TCP bağlantı kurma, TLS el sıkışması, HTTP satırları
- **Custom ClientHello**: JA3/JA4 fingerprint tam kontrolü
- **SNI manipülasyonu**: farklı SNI gönderip farklı Host header yazabilir
- **TLS extension sırası**: hangi extension'ların hangi sırayla gittiğini belirler

### Dezavantajlar
- HTTP/2 için ALPN + h2 frame handling (hyper zaten bunu yapıyor)
- Redirect, cookie, Keep-Alive yönetimi tamamen manuel
- Geliştirme maliyeti yüksek

### Sonuç
Yalnızca TLS parmak izi tam özelleştirmesi gerektiğinde geçerli.
`impcurl` (rquest) zaten bunu soyutluyor.

---

## Seçenek 3: `rquest` Genişletmesi (Mevcut `impcurl` Backendi)

**`rquest`**: fork'd reqwest ile JA3/JA4/HTTP2 fingerprint desteği

### Şu an kullanılan özellikler
- Chrome/Firefox impersonation profilleri
- `--backend impcurl` ile seçilebilir

### Ek geliştirilebilecekler
- Custom impersonation profili tanımı (özel `ImpersonationProfile` struct)
- Rotation: her istek için farklı profil seçimi (rquest'in `Impersonate` enum'u genişletilerek)
- Cookie isolation: her `fonkod` isteği için ayrı cookie jar

---

## Tavsiye

| Hedef | Tavsiye |
|-------|---------|
| SNI / IP-level kontrol | `curl` crate + `CURLOPT_RESOLVE` |
| TLS fingerprint özelleştirme | `rquest` custom profile (impcurl backend genişletmesi) |
| Raw socket / HTTP/2 frame | Sadece kesinlikle gerektiğinde; yüksek maliyet |
| Proxy rotasyonu (TODO #17) | `reqwest` + `Proxy::all()` veya `rquest` proxy desteği yeterli |

**Kısa vadeli aksiyon**: `impcurl` backend'e `--impersonate-profile <name>` flag'i eklemek
(mevcut sabit profil yerine dinamik seçim). Bu, TODO #15 (stealth sertleştirme) ile
birleştirilebilir.

---

*Güncelleme tarihi: 2026-05-04*
