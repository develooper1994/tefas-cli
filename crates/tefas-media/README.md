# tefas-media

TEFAS `getLogo` API yanıtlarını PNG/JPEG dosyalarına dönüştürür.

**Paket adı:** `tefas-media` &nbsp;·&nbsp; **Dizin:** `crates/tefas-media`

---

## Sorumluluk

`getLogo` operasyonu base64-kodlu fon logo görüntülerini JSON içinde döndürür. Bu crate:

- JSON yanıtından base64 blob'larını çıkartır
- `ImageOutputFormat::Png` veya `ImageOutputFormat::Jpeg` olarak çözer
- Belirtilen dizine yazar; zaten mevcut dosyaları atlar

---

## API

```rust
pub fn convert_getlogo_to_images(
    raw: &serde_json::Value,
    outdir: &str,
    format: ImageOutputFormat,
    quality: u8,
    verbose: bool,
) -> anyhow::Result<ImageConvertStats>
```

`ImageConvertStats` alanları:
- `wrote: usize` — diske yazılan dosya sayısı
- `skipped: usize` — zaten mevcut olduğu veya veri bulunmadığı için atlanan sayı
- `errors: usize` — base64 decode veya image encode hatası sayısı

---

## Örnek

```rust
use tefas_media::{ImageOutputFormat, convert_getlogo_to_images};
let stats = convert_getlogo_to_images(
    &api_response,
    "./logos",
    ImageOutputFormat::Png,
    85,
    false,
)?;
println!("Wrote {}, skipped {}", stats.wrote, stats.skipped);
```

---

## İlgili

- `tefas-cli` — `logo --outdir`, `logo --format` ve `logo --quality` flagleri ile bu crate'i kullanır
- `tefas-parser` — HTML sayfadan logo verisi çıkarmak için
