# Dataset Metadata Standard

Bu dosya, `api` ve `fundpage` datasetleri icin metadata standardini tanimlar.

## Schema Dosyalari

- `api/metadata.schema.json`
- `fundpage/metadata.schema.json`

## Kullanim

1. Dataset guncellemesi sonrasi ilgili metadata JSON belgesini schema'ya uygun olarak guncelle.
2. Hash alanlari `sha256` formatinda olmalidir.
3. `capture_date` ISO-8601 date (`YYYY-MM-DD`) formatinda olmalidir.
4. `generated_at` ISO-8601 date-time (`YYYY-MM-DDTHH:MM:SSZ`) formatinda olmalidir.

## Sonraki Adim

`tefas-parser` parity testine metadata dogrulama adimi eklenerek eksik/bozuk metadata durumunda actionable fail verilmelidir.
