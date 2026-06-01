# Operation Quickstart

`xtask` yerine normal kullanımda doğrudan CLI komutları kullanılır.

```bash
cargo run -q -p cli -- --help
```

## Fon Sayfası (fundpage)

```bash
# Birden fazla fonu paralel çek ve parse et
cargo run -q -p cli -- fundpage AC5 TLY AFT

# Ham HTML'leri bir klasöre yedekle
cargo run -q -p cli -- fundpage AC5 TLY --save-html ./html_dump
```

## Takasbank API Sorgusu (query)

```bash
# Tüm operasyonları listele
cargo run -q -p cli -- query --list

# Bir operasyonun giriş alanlarını (payload keys) gör
cargo run -q -p cli -- query --info fonBilgiGetir

# Tek operasyon
cargo run -q -p cli -- query fonBilgiGetir

# Payload alanı override
cargo run -q -p cli -- query fonBilgiGetir --set fonKodu=AC5

# Birden fazla endpoint'i bounded async parallel çağır
cargo run -q -p cli -- query fonBilgiGetir getBanners fonGetiriBazliBilgiGetir --network-concurrency 8
```

### Query Smoke Profilleri

```bash
# Baslangic (WAF guvenli)
cargo run -q -p cli -- query fonBilgiGetir --network-concurrency 2 --quiet

# Dengeli
cargo run -q -p cli -- query fonBilgiGetir fonGetiriBazliBilgiGetir --network-concurrency 4 --quiet

# Agresif (sadece ortama gore)
cargo run -q -p cli -- query fonBilgiGetir fonGetiriBazliBilgiGetir fonBuyuklukBazliBilgiGetir --network-concurrency 8 --quiet
```

## Ham HTML Çek (fetch)

```bash
# Birden fazla URL'yi paralel çek ve bir dizine kaydet
cargo run -q -p cli -- fetch https://www.tefas.gov.tr https://www.takasbank.com.tr \
  --output ./downloads

# Fetch için bounded async parallel
cargo run -q -p cli -- fetch https://www.tefas.gov.tr https://www.takasbank.com.tr https://www.takasbank.com.tr/tr/veri \
  --network-concurrency 8 --output ./downloads
```

### Fetch Smoke Profilleri

```bash
# Baslangic (WAF guvenli)
cargo run -q -p cli -- fetch https://www.tefas.gov.tr https://www.takasbank.com.tr --network-concurrency 2 --output ./downloads

# Dengeli
cargo run -q -p cli -- fetch https://www.tefas.gov.tr https://www.takasbank.com.tr https://www.takasbank.com.tr/tr/veri --network-concurrency 4 --output ./downloads

# Cevap kalitesi kontrolu
grep -R "Request Rejected" ./downloads/*.html
```

## Yerel HTML Parse Et (parse)

```bash
# Birden fazla HTML dosyasını parse et
cargo run -q -p cli -- parse file1.html file2.html
```

## Logo İndir (logo)

```bash
# PNG olarak kaydet (varsayılan)
cargo run -q -p cli -- logo AC5 TLY

# JPEG, özel dizin ve kalite
cargo run -q -p cli -- logo AC5 TLY --format jpeg --quality 90 --outdir ./logos
```

## Backend / TLS Seçimi

```bash
# Hyper + NativeTLS
cargo run -q -p cli -- --backend hyper --tls nativetls query fonBilgiGetir

# curl-impersonate + Windows persona
cargo run -q -p cli -- --backend impcurl --persona desktop-windows fundpage AC5

# Manuel impersonate profili
cargo run -q -p cli -- --backend impcurl --impersonate chrome136 fundpage AC5
```

## Shell Completion

```bash
# Bash completion scripti üret
cargo run -q -p cli -- completion bash >> ~/.bash_completion
```

Devam: [reference.md](reference.md)

Performans/darboğaz analizi için: [profiling.md](profiling.md)
