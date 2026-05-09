# tefas-cli

TEFAS / Takasbank fon veri CLI binary'si.

**Paket adı:** `tefas-cli` &nbsp;·&nbsp; **Dizin:** `crates/tefas-cli`

---

## Sorumluluk

Tüm workspace crate'lerini birleştiren giriş noktası. Sorumluluklar:

1. Subcommand seçimi (`fundpage`, `query`, `fetch`, `parse`, `logo`, `completion`)
2. `AppConfig` inşası (backend/TLS/retry/auth global flagleri)
3. Smart Defaults — `curl-impersonate` mevcutsa `fund` ve `fetch` için otomatik WAF bypass
4. `NetworkClient` ile HTTP isteği gönderme
5. `--format json|humanize` ile sonucu biçimlendirip yazdırma
6. `logo` işlemi için `tefas-media` ile görüntü dönüşümü
7. Preflight session warmup

---

## Kullanım

```bash
# Fon sayfasını çek ve parse et (WAF bypass otomatik)
tefas fundpage AC5

# Takasbank API sorgusu
tefas query fonBilgiGetir

# Birden fazla operasyon — sonuçlar birleşir
tefas query fonBilgiGetir getBanners

# Payload override
tefas query fonBilgiGetir --set fonKodu=AC5

# Legacy ASMX operasyonu
tefas query --old getAllFunds

# Ham HTML çek
tefas fetch https://www.tefas.gov.tr/tr/fon-detayli-analiz/AC5

# Yerel HTML parse et
tefas parse tefas_fetched.html

# Logo PNG olarak kaydet
tefas logo AC5 TLY --format png

# Shell completion
tefas completion bash
```

Backend ve TLS seçimi için global flagler subcommand'dan önce yazılır:

```bash
# curl-impersonate + Windows persona
tefas --backend impcurl --persona desktop-windows fundpage AC5

# Hyper + NativeTLS
tefas --backend hyper --tls nativetls query fonBilgiGetir
```

Tam flag referansı: [docs/operations/reference.md](../../docs/operations/reference.md)  
Hızlı başlangıç örnekleri: [docs/operations/quickstart.md](../../docs/operations/quickstart.md)

---

## Build

```bash
CARGO_HTTP_CAINFO=/etc/ssl/certs/ca-certificates.crt \
  cargo build --release -p tefas-cli
# Çıktı: target/release/tefas-cli
```

---

## İlgili

- [docs/architecture.md](../../docs/architecture.md) — crate bağımlılık grafiği
- `tefas-api` — operasyon descriptor'ları
- `tefas-network` — HTTP backend
