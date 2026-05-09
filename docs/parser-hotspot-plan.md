# Parser Hotspot Plan

Bu belge, `tefas-parser` içinde kalan regex/string tarama hotspot'larini daha yapisal bir yaklasimla ele almak icin uygulama planidir.

## Kapsam

Hedef kod yolu agirlikli olarak [../crates/parser/src/fund_page/mod.rs](../crates/parser/src/fund_page/mod.rs) icindeki su fonksiyonlarda toplanir:

- `parse_html_text`
- `extract_p_pairs`
- `extract_next_f_string_payloads`
- `collect_json_fields`
- `extract_rsc_fast_fields`
- `next_p_value_after_label` / `collect_next_p_values_after_labels`

Remote perf raporlarinda kalan maliyetin ana govdesi `regex_automata::*` ve tekrarlayan string scan'lerden geliyor. Son denemeler de sunu dogruladi:

- dusuk payli `Value::Array` erisimi veya allocation mikrolari kazanc getirmedi
- RSC sayfalarda bile HTML tabanli `p_pairs` fallback'i hala davranissal olarak gerekli
- parity-safe gorunen tarayici refactor'lari remote perf'te kolayca regress edebiliyor

## Mevcut Sorunlar

### 1. Ayni dokuman birden fazla kez taraniyor

`parse_html_text` ayni girdi uzerinde su taramalari tekrarliyor:

- `text.contains("__next_f.push")`
- `INFO_SCOPE_RE.find(text)`
- `extract_p_pairs(text)`
- `DETAILS_VIEW_TABLE_RE.captures(text)`
- `next_p_value_after_label(...)` tabanli ardil taramalar
- RSC fallback icin `extract_next_f_string_payloads(text)` + `collect_json_fields(...)`

Bu, tek tek ucuz gorunen ama toplu etkisi buyuyen bir pattern olusturuyor.

### 2. Regex yardimcilari karar verici katmanda daginik

Regex kullanan yardimcilar yalnizca extraction yapmiyor; bazi durumlarda parse stratejisini de belirliyor. Ornekler:

- `extract_p_pairs` hem preferred hem fallback modunu icinde tasiyor
- `extract_rsc_fast_fields` payload tarama + alan secme + decode + normalize islerini birlikte yapiyor
- `parse_html_text` icinde hangi fallback'in ne zaman calisacagi lineer olarak karisiyor

### 3. Alan bazli okumalar, payload bazli scanner katmani olmadan yapiliyor

RSC akisinda mevcut yol su sekilde:

1. `extract_next_f_string_payloads` ile payload'lari topla
2. her payload icin `collect_json_fields`
3. her alan icin `extract_json_*_field_from_fields` ile lineer lookup yap

Bu model parity-safe ama hem tekrarli hem de belirli hedef alanlar icin gereksiz genis tarama yapiyor.

## Calisma Hipotezi

Olcumlenebilir bir sonraki kazanc, dusuk seviyeli mikro degisikliklerden degil, tarama maliyetini iki asamada azaltmaktan gelecek:

1. karar verici katmanda dokumani bir kez siniflandirmak
2. her sinif icin daha dar ve amaca ozel scanner akislari kullanmak

Bu, tam bir regex temizligi anlamina gelmiyor. Hedef, en pahali tam-belge regex'leri daha dar scope'lara cekmek ve tekrarli alan taramalarini ortak ara temsillerle azaltmak.

## Uygulama Plani

### Asama 1. Belge siniflandirma katmani

`parse_html_text` basinda hafif bir `DocumentKind` ve `DocumentSlices` katmani eklenmeli.

Onerilen minimal tipler:

```rust
enum DocumentKind {
    LegacyHtml,
    NextJsRsc,
    Hybrid,
}

struct DocumentSlices<'a> {
    full: &'a str,
    info_scope: Option<&'a str>,
    chart_scope: &'a str,
    has_rsc_frames: bool,
}
```

Buradaki amac parser'lari ayirmak degil; once tekrarli `find` ve `contains` kararlarini tek yerde toplamak.

Basari kriteri:

- `parse_html_text` icinde `text.contains`, `text.find`, scope secimi ve page-type kontrolu tek blokta toplansin
- davranis degismesin

### Asama 2. HTML label scanner'ini regex'ten ayir

`extract_p_pairs`, `next_p_value_after_label` ve `collect_next_p_values_after_labels` birbirine yakin problem cozuuyor: sirasal `<p>` label/value akisi.

Onerilen yon:

- `P_PAIR_RE` ve `RETURN_P_RE` etrafinda tek bir `PLabelScanner` abstraction'i tasarla
- ilk iterasyonda regex'i tamamen silme
- once tek bir interface altina topla, sonra ic implementasyonu degistirilebilir hale getir

Ornek interface:

```rust
struct PLabelScanner<'a> { /* precomputed p text stream */ }

impl<'a> PLabelScanner<'a> {
    fn build(text: &'a str) -> Self;
    fn explicit_pairs(&self) -> HashMap<String, String>;
    fn next_value_after_label(&self, label: &str, decode: bool) -> Option<String>;
    fn next_values_after_labels(&self, labels: &[&str], decode: bool) -> HashMap<String, String>;
}
```

Kazanc beklentisi:

- ayni `<p>` stream'i return/profile fallback'lerinde yeniden taranmaz
- `parse_html_text` icindeki fallback mantigi kisa ve test edilebilir olur

Risk:

- onceki full-document scanner denemesi regress etti; bu nedenle ilk hedef implementasyon degil, interface + parity harness olmali

### Asama 3. RSC payload scanner'ini alan-toplayici yerine hedefli yap

`collect_json_fields` tum field/value ciftlerini cikariyor, sonra cogu alan icin tekrar lineer aranıyor.

Burada hedef tam JSON parser'a gecmek degil. Once dar hedefli bir API tanimlanmali:

```rust
struct RscPayloadView<'a> {
    raw: &'a str,
}

impl<'a> RscPayloadView<'a> {
    fn find_string(&self, keys: &[&str]) -> Option<String>;
    fn find_number(&self, keys: &[&str]) -> Option<f64>;
    fn find_link(&self, keys: &[&str]) -> Option<String>;
}
```

Ikinci iterasyonda `collect_json_fields` sadece fallback/debug araci olarak kalabilir. Sicak yol, hedef anahtar listeleri icin calismali.

Ilk hedef alanlar:

- `fonKodu` / `fundCode`
- `isinKodu` / `isin`
- `fonToplamDeger` / `fundTotalValue`
- `pazarPayi` / `marketShare`
- `fonRiskDegeri` / `riskValue`
- profile enrichment adaylari (`basIsSaat`, `sonIsSaat`, `girisKomisyonu`, `kategoriDerece`)

### Asama 4. Chart/allocation path'ini yalnizca scope daraltmayla ele al

Son deneme gosterdi ki `normalize_data` uzerinden mikro optimizasyon anlamli degil. Bu alan icin bir sonraki mantikli adim yalnizca scope ve cikis kontrolu:

- `chart_scope` belirleme tek yerde yapilsin
- `find_series_blocks` ve `SERIES_INIT_RE` fallback zinciri tek helper'a tasinsin
- allocation ve return extraction icin ayri scan tetiklemek yerine ortak chart snapshot kullanilsin

## Oncelik Sirasi

1. `DocumentKind` + `DocumentSlices`
2. `PLabelScanner` interface + mevcut davranisi koruyan adaptor
3. `RscPayloadView` interface + hedefli key lookup denemesi
4. chart block snapshot yardimcisi

## Guardrail'ler

Her asama su sira ile ilerlemeli:

1. lokal unit/integration test
2. `compare_fundpage_datasets`
3. gerekirse `cargo run -q -p tefas-cli -- parse ../datasets/fundpage/html/*.html`
4. sadece bundan sonra `cargo xtask tefas bench --no-samply --cargo-profile profileDebug`

Yerelde `perf` veya `samply` calistirilmamali.

## Basari Olcutleri

- parity bozulmadan current winning remote band'in altina inmek
- hedef karsilastirma bandi: yaklasik `2071.28 / 2148.88 / 2275.13 ms`
- yeni refactor'lar en az bir 3-run karsilastirmada median bazinda anlamli kazancli olmali

## Yapilmamasi Gerekenler

- `extract_p_pairs`'i RSC icin toptan kapatmak
- `normalize_data` veya benzeri dusuk payli serde kollari etrafinda mikro arama yapmak
- parity-safe olmadan genis scanner refactor'u remote'a tasimak

## Beklenen Son Cikti

Bu plandan sonra parser optimizasyonu artik "hangi regex'i kaldiralim" seviyesinden cikip, iki acik akis uzerinden ilerlemeli:

- HTML label scanner hatti
- RSC payload scanner hatti

Bu iki hat ayri olcumlenebilir hale gelmeden sonraki perf turleri muhtemelen yine gürültü ve regress karisimi uretecek.