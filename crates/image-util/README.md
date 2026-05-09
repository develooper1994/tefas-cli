# image-util

Generic image conversion utility crate.

## Kapsam

- base64 payload decode (data URI dahil)
- PNG/JPEG cikti formati
- image dosyasina yazma
- toplu donusum istatistikleri

TEFAS `getLogo` gibi API'ye ozel alan esleme bu crate'te degil, adapter crate'te olmalidir.

## Build

```bash
cd shared
cargo check -p image-util
```
