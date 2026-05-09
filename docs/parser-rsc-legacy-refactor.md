# Parser RSC/Legacy Refactor Outline

Bu belge, [../crates/parser/src/fund_page/mod.rs](../crates/parser/src/fund_page/mod.rs) icindeki `parse_html_text` akisini daha net iki parser yoluna ayirmak icin mimari taslagi tarif eder.

## Problem Tanimi

Bugunku durumda `parse_html_text` tek fonksiyon icinde su sorumluluklari birlikte tasiyor:

- sayfa tipi tespiti (`__next_f.push`)
- scope secimi (`info_scope`, `chart_scope`)
- indicator/profile/return/allocation extraction
- legacy table/`<p>` path'i
- RSC enrichment ve RSC fast fallback
- post-processing (`fon_adi` fallback, `kap_bilgi_adresi`, grouped/meta olusturma)

Bu, iki farkli gercegi tek govdede birlestiriyor:

- Legacy HTML sayfalarda veri gorunur DOM label/value ciftlerinden geliyor
- RSC sayfalarda veri agirlikli olarak `__next_f.push(...)` payload'larinda geliyor, ama bazi profile alanlari icin HTML fallback hala gerekli

Dolayisiyla hedef "RSC sayfada legacy parser'i tamamen kapatmak" degil. Hedef, karar verici akisi ve veri kaynaklarini acik sinirlara ayirmak.

## Tasarim Hedefleri

### 1. Dispatcher ile parser implementasyonunu ayir

`parse_html_text` en ustte kalsin, ama yalnizca su isi yapsin:

1. belgeyi siniflandir
2. uygun parser yolunu cagir
3. ortak merge/finalize adimlarini uygula

### 2. RSC ve legacy parse kaynaklarini acikca isimlendir

Bugun `pairs`, `p_pairs`, `rsc_fast`, `enrich_rsc_profile_from_payloads` ayni scope'ta yan yana duruyor. Bunun yerine kaynaklar tip seviyesinde ayrilmali.

### 3. Fallback mantigini yonlu yap

Mevcut kodda fallback'ler lineer olarak govdeye dagilmis durumda. Refactor sonrasi fallback yonu acik olmali:

- once primary source
- sonra compatible secondary source
- en sonda targeted fallback

## Onerilen Yapi

### Ust seviye akış

```rust
pub fn parse_html_text(text: &str) -> (Value, Value) {
    let doc = FundDocument::classify(text);

    let parsed = match doc.kind {
        DocumentKind::LegacyHtml => parse_legacy_document(&doc),
        DocumentKind::NextJsRsc => parse_rsc_document(&doc),
        DocumentKind::Hybrid => parse_hybrid_document(&doc),
    };

    finalize_document(parsed)
}
```

### Onerilen tipler

```rust
struct FundDocument<'a> {
    kind: DocumentKind,
    full_text: &'a str,
    info_scope: Option<&'a str>,
    chart_scope: &'a str,
}

enum DocumentKind {
    LegacyHtml,
    NextJsRsc,
    Hybrid,
}

struct ParseAccumulator {
    fields: Map<String, Value>,
    meta: Map<String, Value>,
}
```

`Hybrid`, pratikte RSC frame tasiyan ama HTML fallback'e hala ihtiyac duyan mevcut fundpage davranisini temsil eder. Bu, bugunku gercekle en uyumlu modeldir.

## Parser Sorumluluklari

### `parse_legacy_document`

Bu parser yalnizca su kaynaklari okur:

- `extract_indicator_pairs`
- `extract_p_pairs` veya onun yerine gelecek `PLabelScanner`
- legacy details table (`DETAILS_VIEW_TABLE_RE` + row/cell extraction)
- `next_p_value_after_label` tabanli targeted fallback'ler
- chart extraction

Bu parser RSC payload taramaz.

### `parse_rsc_document`

Bu parserin primary source'u RSC payload'tur:

- `extract_next_f_string_payloads`
- `extract_rsc_fast_fields`
- `enrich_rsc_profile_from_payloads`

Ama tamamen HTML'siz varsayim yapmaz. Yalnizca gerekli alanlar icin sinirli HTML fallback kullanir:

- profile label/value yardimcilari
- return / allocation icin chart helpers

Buradaki kural: HTML fallback parser'in primary modeli degil, tamamlayici dependency'si olur.

### `parse_hybrid_document`

Ilk iterasyonda `parse_hybrid_document`, fiilen bugunku davranisin isimlendirilmis hali olabilir:

1. legacy-compatible HTML extraction
2. RSC enrichment
3. RSC fast fallback

Bu ara asama onemli; cunku davranisi korurken kontrol akisina isim vermeyi saglar.

## Merge Kurallari

Refactor sonrasi merge semantigi tek yerde toplanmali. Onerilen oncelik:

1. explicit table / explicit HTML pair
2. normalized label fallback
3. RSC enrichment
4. RSC fast fallback
5. targeted single-field fallback

Bugun bu mantik govdeye dagilmis durumda. `merge_if_missing`, `merge_decoded_string`, `merge_number_if_missing` gibi kucuk yardimcilarla acik hale getirilmeli.

## Dosya Duzeni Onerisi

Ilk etapta tek dosya kalabilir; ama sembolik ayrim yapilmali. Ikinci etapta su parcalama mantikli:

- `fund_page/mod.rs`: public API + dispatcher
- `fund_page/document.rs`: `FundDocument`, `DocumentKind`, scope secimi
- `fund_page/legacy.rs`: legacy/html extraction
- `fund_page/rsc.rs`: RSC payload extraction + enrichment
- `fund_page/chart.rs`: Highcharts/allocation/return yardimcilari
- `fund_page/merge.rs`: accumulator + merge kurallari

Bu dagilim, hem perf denemelerini lokalize eder hem de parity regress oldugunda sorumlu parser hattini netlestirir.

## Iterasyon Sirasi

### Faz 1. Yapisal ayirma, davranis ayni

- `FundDocument` ve `DocumentKind` ekle
- `parse_html_text` dispatcher haline gelsin
- mevcut kod bloklarini private helper'lara tasi
- parity degismemeli

### Faz 2. Merge semantigini merkezilestir

- `ParseAccumulator` ekle
- `res.insert` daginikligini azalt
- fallback onceligini kod seviyesinde okunur hale getir

### Faz 3. Scanner implementasyonlarini parser bazinda ayir

- legacy tarafinda `PLabelScanner`
- RSC tarafinda `RscPayloadView`

### Faz 4. Ancak bundan sonra perf odakli alt implementasyon degisikligi yap

Bu siralama onemli. Daha onceki regress'lar, yapi ve mikro-opt denemelerinin ayni patch icinde karismasindan dogdu.

## Test Stratejisi

Refactor boyunca asgari guvence:

- `cargo test -p tefas-parser`
- `cargo test -p tefas-parser --test compare_datasets compare_fundpage_datasets`
- gerekirse tum fixture parse turu

Refactor parity-safe olduktan sonra remote perf karsilastirmasi yapilmali.

## Beklenen Kazanclar

- `parse_html_text` okunabilir bir dispatcher'a donusur
- RSC/legacy davranis farklari kodda aciklasir
- yeni perf denemeleri "butun parser" yerine ilgili parser hattinda yapilir
- parity regress oldugunda hata alani daralir

## Kirmizi Cizgiler

- RSC sayfalarda HTML fallback'i dogrudan silme
- refactor ile perf denemesini ayni patch'te birlestirme
- merge onceligini sessizce degistirme

## Karar Ozeti

Bu parser icin bir sonraki saglikli adim, dogrudan yeni mikro-opt aramak degil. Once `parse_html_text` icindeki gercek iki veri kaynagini mimari seviyede isimlendirip ayirmak gerekiyor. Bu yapilmadan atilan perf adimlari tekrar ayni karisiklik icinde olculmeye devam edecek.