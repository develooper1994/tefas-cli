# tefas-api

TEFAS / Takasbank JSON API operasyon tanımlayıcıları.

**Paket adı:** `tefas-api` &nbsp;·&nbsp; **Dizin:** `crates/api`

---

## Sorumluluk

Bu crate **kasıtlı olarak ağ bağımlılığı içermez**. Yalnızca şunları tanımlar:

- `Operation` — 32 güncel Takasbank/TEFAS API operasyonu (`dagilimSiraliGetirT`, `fonBilgiGetir`, …)
- `OperationOld` — Legacy operasyon seti (`getAllFunds`, `getAllFundAnalyzeData`, …)
- `OperationSpec` — Her operasyon için `endpoint` ve `referer`
- `FundSummary` + `normalize_fund_summary_list` — Fon özeti normalizasyonu

**Ne** isteneceğini bilir; **nasıl** gönderileceğini bilmez. Yönlendirme `tefas-cli`'ın sorumluluğundadır.

---

## Örnek

```rust
use tefas_api::Operation;

let spec = Operation::FonBilgiGetir.spec();
println!("POST {}", spec.endpoint);
println!("Referer: {}", spec.referer);

let payload = Operation::FonBilgiGetir.default_payload();
```

---

## İlgili

- [docs/api/INPUT_OUTPUT_FIELDS.md](../../docs/api/INPUT_OUTPUT_FIELDS.md) — alan bazlı referans
- [docs/operations/reference.md](../../docs/operations/reference.md) — CLI flag referansı
- `tefas-network` — bu spec'leri kullanarak HTTP isteği yapan katman
