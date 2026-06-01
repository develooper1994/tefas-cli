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

## Concurrency Matrix (Yerel, Hizli Kontrol)

Fundpage parser tarafinda concurrency etkisini hizli gormek icin yerel fixture setiyle asagidaki matrix calistirilabilir.

```bash
set -euo pipefail
files=$(find datasets/fundpage/html -maxdepth 1 -type f -name '*.html' | sort)

measure() {
  c="$1"
  vals=""
  for i in 1 2 3; do
    t=$(/usr/bin/time -f "%e" \
      cargo run -q -p cli -- --parse-concurrency "$c" parse $files --output "/tmp/tefas-parse-$c-$i.json" \
      2>&1 >/dev/null | tail -n 1)
    vals+="$t\n"
  done
  printf "%b" "$vals" | sed '/^$/d' | sort -n | awk 'NR==2{print $1}'
}

echo "concurrency,median_seconds"
for c in 1 2 4 8; do
  echo "$c,$(measure "$c")"
done
```

Yorumlama:

- Ilk kosu derleme maliyeti icerebilir; bu nedenle 3 tekrar + medyan kullan.
- 1 -> 2 artisinda net kazanc beklenir.
- 4 ve 8 seviyelerinde kazanc ortam ve CPU cekirdek sayisina gore degisken olabilir.
- Bu matrix parser CPU olcegini gosterir; fundpage fetch + parse birlesik etkisi icin remote bench ile birlikte degerlendirilmelidir.

## End-to-End Fundpage Matrix (Network x Parse)

Fundpage akisinda network ve parse concurrency etkisini birlikte gormek icin kucuk bir grid olcumu:

```bash
cargo xtask tefas fundpage-matrix \
  --network 2,4 \
  --parse 2,8 \
  --runs 3 \
  --warmup 1 \
  --codes AC5,TLY,AFT \
  --output /tmp/tefas-fundpage-matrix.csv
```

Bu komut medyan sureleri hesaplayip `network,parse,median_seconds` formatinda CSV uretir.

Manuel olcum alternatifi:

```bash
set -euo pipefail

measure() {
  net="$1"
  parse="$2"
  /usr/bin/time -f "%e" \
    cargo run -q -p cli -- \
      --network-concurrency "$net" \
      --parse-concurrency "$parse" \
      fundpage AC5 TLY AFT \
      --output "/tmp/tefas-fundpage-n${net}-p${parse}.json" \
    2>&1 >/dev/null | tail -n 1
}

echo "network,parse,seconds"
for net in 2 4; do
  for parse in 2 8; do
    echo "$net,$parse,$(measure "$net" "$parse")"
  done
done
```

Yorumlama:

- `network` artisina karsin iyilesme varsa darbogazin bir kismi I/O tarafindadir.
- `parse` artisina karsin iyilesme varsa CPU parse tarafinda kazanilacak alan vardir.
- En iyi kombinasyon ortama gore degisir; tek bir sabit deger yerine kucuk grid ile secim yap.
- Dis ag veya WAF degiskenligi nedeniyle sonuclari en az 3 tekrar + medyan ile degerlendirmek daha guvenlidir.

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
