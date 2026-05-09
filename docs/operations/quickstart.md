# Operation Quickstart

## Fon Sayfası (fundpage)

```bash
# Birden fazla fonu paralel çek ve parse et
cargo run -q -p tefas-cli -- fundpage AC5 TLY AFT

# Ham HTML'leri bir klasöre yedekle
cargo run -q -p tefas-cli -- fundpage AC5 TLY --save-html ./html_dump
```

## Takasbank API Sorgusu (query)

```bash
# Tüm operasyonları listele
cargo run -q -p tefas-cli -- query --list

# Bir operasyonun giriş alanlarını (payload keys) gör
cargo run -q -p tefas-cli -- query --info fonBilgiGetir

# Tek operasyon
cargo run -q -p tefas-cli -- query fonBilgiGetir

# Payload alanı override
cargo run -q -p tefas-cli -- query fonBilgiGetir --set fonKodu=AC5
```

## Ham HTML Çek (fetch)

```bash
# Birden fazla URL'yi paralel çek ve bir dizine kaydet
cargo run -q -p tefas-cli -- fetch https://www.tefas.gov.tr https://www.takasbank.com.tr \
  --output ./downloads
```

## Yerel HTML Parse Et (parse)

```bash
# Birden fazla HTML dosyasını parse et
cargo run -q -p tefas-cli -- parse file1.html file2.html
```

## Logo İndir (logo)

```bash
# PNG olarak kaydet (varsayılan)
cargo run -q -p tefas-cli -- logo AC5 TLY

# JPEG, özel dizin ve kalite
cargo run -q -p tefas-cli -- logo AC5 TLY --format jpeg --quality 90 --outdir ./logos
```

## Backend / TLS Seçimi

```bash
# Hyper + NativeTLS
cargo run -q -p tefas-cli -- --backend hyper --tls nativetls query fonBilgiGetir

# curl-impersonate + Windows persona
cargo run -q -p tefas-cli -- --backend impcurl --persona desktop-windows fundpage AC5

# Manuel impersonate profili
cargo run -q -p tefas-cli -- --backend impcurl --impersonate chrome136 fundpage AC5
```

## Shell Completion

```bash
# Bash completion scripti üret
cargo run -q -p tefas-cli -- completion bash >> ~/.bash_completion
```

Devam: [reference.md](reference.md)

Performans/darboğaz analizi için: [profiling.md](profiling.md)
