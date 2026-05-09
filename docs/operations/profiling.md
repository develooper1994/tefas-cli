# Profiling Workflow

Bu doküman parser performans darboğazlarını bulmak ve iteratif olarak iyileştirmek için standart iş akışını tanımlar.

## Amaç

- Darboğazı tahminle değil ölçümle bulmak.
- Her iterasyonda tek ana değişiklik yapıp etkisini izole etmek.
- Doğruluk (parity) bozulmadan throughput/latency iyileştirmek.

## Operasyonel Notlar

- TODO/handoff maddeleri artik `tefas/README.md` icinde tutulmuyor; operasyonel takip bu dokumandadir.
- Son dogrulanmis gate: `cargo test -p tefas-parser` yesil.
- Son benchmark komutu: `cargo bench -p tefas-parser --bench parse_document_bench -- --sample-size 10`.
- WSL ortami icin `perf record/report` calistirmak gerekiyorsa kernel uyumlu `linux-tools-$(uname -r)` paketi yuklenmelidir.

## Önkoşullar

```bash
command -v samply
samply --version
```

Remote-only notu:

- Bu workflow'da `perf` ve `samply` sadece uzak Linux host üzerinde çalıştırılır.
- Yerelde profiling akışı doğrudan `cargo xtask tefas bench` üzerinden yürütülür.
- `cargo xtask tefas bench` tüm repo ile birlikte datasetleri (`../datasets/fundpage/html`) remote host'a rsync eder ve dataset varlığını doğrular.

Tek komutla uçtan uca ölçüm/profiling döngüsü:

```bash
cargo xtask tefas bench
```

Script çıktıları yerelde `target/perf-artifacts-remote/` altına senkronlanır ve remote tarafta sırayla:

- parser gate testlerini çalıştırır,
- criterion benchmark baseline kaydeder,
- samply profil + hotspot özeti üretir (varsayılan),
- varsa `perf stat`, `perf record/report` ve `cargo-flamegraph` çıktısını ekler.

Not: benchmark derlemeleri varsayılan olarak `profiling` cargo profilinde çalışır.
Gerekirse `--cargo-profile release` ile değiştirilebilir.

## perf Ciktisini Hizli Alma

En kolay yol (tek komut):

```bash
cargo xtask tefas bench
```

Son kosunun perf raporunu okumak:

```bash
latest="$(find target/perf-artifacts-remote -mindepth 1 -maxdepth 1 -type d | sort | tail -n 1 | xargs basename)"
sed -n '1,160p' "target/perf-artifacts-remote/$latest/perf_report.txt"
```

Notlar:

- `--no-samply` ile samply kapatılabilir; `--flamegraph` ek olarak açılabilir (ikisi de remote çalışır).
- `--filter EXPR` ile hedef kısıtlanır (varsayılan: `parse_document/AC5`).
- `--cargo-profile NAME` ile benchmark profilini seçebilirsin (varsayılan: `profiling`).
- Rapor dosyalari: `perf_stat.txt`, `perf.data`, `perf_report.txt`, `flamegraph.svg`.

Opsiyonel:

```bash
command -v llvm-profdata || true
```

Debug sembolleriyle release optimizasyonu (önerilen ilk adım):

```bash
RUSTFLAGS="-C debuginfo=2" cargo build --release
```

Alternatif kalıcı yaklaşım:

```toml
# .cargo/config.toml
[profile.profiling]
inherits = "release"
debug = true
```

Sonra:

```bash
cargo build --profile profiling
```

## Baseline Alma

Önce doğruluk kapısı:

```bash
CARGO_HTTP_CAINFO=/etc/ssl/certs/ca-certificates.crt \
  cargo test -p tefas-parser
```

Sonra benchmark/perf baseline (remote):

```bash
cargo xtask tefas bench \
  --filter parse_document/AC5 \
  --warmup 1 \
  --measure 3
```

Samply profil çıktısı üret (remote, varsayılan açık):

```bash
cargo xtask tefas bench --filter parse_document/AC5 --samply
```

Samply çıktısını hızlı özetlemek için:

```bash
latest="$(find target/perf-artifacts-remote -mindepth 1 -maxdepth 1 -type d | sort | tail -n 1)"
cargo xtask tefas samply-summary "$latest/profile.json.gz" --top 20
```

## Hotspot Analizi

`profile.json.gz` için öncelik sırası:

1. `parse_html_text` içinde toplam CPU zamanı en yüksek bloklar
2. Yüksek frekanslı regex çağrıları (`captures_iter`, `replace_all`, `find_iter`)
3. Gereksiz string kopyaları (`to_string`, çoklu normalize geçişleri)

Analizde her iterasyonda sadece ilk 1-2 hotspot hedeflenir.

## İteratif Döngü (Measure -> Change -> Verify)

### 1) Measure

- `cargo test -p tefas-parser`
- `cargo xtask tefas bench ...`

### 2) Change

- Tek bir ana performans değişikliği uygula.
- Davranış değişikliği gerekiyorsa fallback koru.

### 3) Verify

```bash
CARGO_HTTP_CAINFO=/etc/ssl/certs/ca-certificates.crt cargo test -p tefas-parser
cargo xtask tefas bench --filter parse_document/AC5 --warmup 1 --measure 3
```

## Gate Kriterleri

Bir iterasyon "başarılı" sayılmak için:

- Throughput artmalı (veya en azından düşmemeli)
- P95/P99 gecikme kötüleşmemeli
- Dataset parity testleri bozulmamalı
- Bellek/alloc davranışı belirgin kötüleşmemeli

## Durdurma Kriteri (Plateau)

Aşağıdakilerden biri olduğunda bir sonraki TODO maddesine geç:

- Arka arkaya 2 iterasyonda net kazanç < %2
- Yapılan değişiklikler doğruluk kapısını bozuyor
- Hotspot artık parser dışında (I/O veya benchmark overhead)

## Kayıt Şablonu

Her iterasyonda aşağıdaki özet TODO içinde güncellenir:

- Değişiklik: ne optimize edildi?
- Önce: benchmark + hotspot özeti
- Sonra: benchmark + hotspot özeti
- Karar: devam / geri al / sonraki maddede ilerle

## İlgili Dosyalar

- `crates/parser/src/fund_page/mod.rs`
- `crates/parser/benches/parse_document_bench.rs`
- `crates/parser/tests/compare_datasets.rs`
- `TODO.md`
