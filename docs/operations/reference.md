# Operation Reference

## Subcommandlar

| Subcommand | Kısa açıklama |
|---|---|
| `fundpage <CODE>...` | Fon sayfalarını çek ve parse et (paralel) |
| `query [OP]...` | API operasyonları, listeleme ve keşif |
| `fetch <URL>...` | Ham HTML'leri çek (paralel) |
| `parse <FILE>...` | Yerel HTML dosyalarını parse et (paralel) |
| `logo <CODE>...` | Fon logolarını PNG/JPEG olarak kaydet |
| `completion <SHELL>` | Shell completion scripti üret |

## fundpage

```
tefas fundpage <CODE>... [--output <FILE>] [--save-html <DIR>]
```

- `CODE...` — Bir veya daha fazla fon kodu.
- `--output` — JSON çıktı dosyası.
- `--save-html` — Ham HTML dokümanlarının kaydedileceği dizin.

## query

```
tefas query [<OP>...] [--list] [--info <OP>] [--old <OP>...] [--set KEY=VALUE]... [--payload <JSON>] [--format json|humanize]
```

- `--list` — Mevcut tüm operasyonları ve açıklamalarını listeler.
- `--info <OP>` — Belirtilen operasyonun giriş alanlarını (payload anahtarları) gösterir.
- `<OP>` — Operasyon adı.

## fetch

```
tefas fetch <URL>... [--output [<PATH>...]] [--skip-preflight]
```

- `URL...` — Çekilecek bir veya daha fazla URL.
- `--output` — Çıktı yolu/ yolları. Tek dizin verilirse URL bazlı otomatik adlandırır; N URL için N dosya da verilebilir.
- `--skip-preflight` — Session warmup adımını atlar.

## parse

```
tefas parse <FILE>... [--output <FILE>]
```

- `FILE...` — Girdi HTML dosyaları.

## logo

```
tefas logo <CODE>... [--outdir <DIR>] [--format png|jpeg] [--quality <1-100>]
```

- `CODE...` — Bir veya daha fazla fon/üye kodu.
- `--outdir` — Görüntülerin yazılacağı dizin (varsayılan: `./logos`).
- `--format` — PNG (varsayılan) veya JPEG.
- `--quality` — JPEG kalitesi 1–100 (varsayılan: 85).

## Global Flagler

Tüm subcommandlar bu flagleri destekler (subcommand'dan **önce** yazılır):

| Flag | Ortam değişkeni | Varsayılan | Açıklama |
|---|---|---|---|
| `--timeout <N>` | `TEFAS_TIMEOUT` | 25 | İstek timeout'u (saniye) |
| `--backend wreq\|reqwest\|hyper\|impcurl` | `TEFAS_BACKEND_NETWORK` | `wreq` | HTTP client backend |
| `--tls rustls\|nativetls\|insecure` | `TEFAS_BACKEND_TLS` | `rustls` | TLS backend |
| `--persona desktop-windows\|desktop-macos\|android\|ios` | `TEFAS_PERSONA` | — | Tarayıcı/cihaz persona preset'i |
| `--impersonate <PROFILE>` | `TEFAS_IMPCURL_IMPERSONATE` | — | curl-impersonate profili (ör. `chrome136`) |
| `--proxy <URL>` | `TEFAS_PROXY` | — | HTTP/HTTPS/SOCKS5 proxy URL |
| `--base-url <URL>` | `TEFAS_BASEURL` | `https://www.tefas.gov.tr` | TEFAS base URL |
| `--pretty` | — | true | JSON pretty-print |
| `--quiet` / `-q` | — | false | Bilgi mesajlarını gizle |

### Backend/TLS Uyum Notlari

- `--backend wreq` secildiginde `--tls` degeri etkisizdir (`wreq` kendi BoringSSL yiginini kullanir).
- `--backend hyper` + `--tls insecure` desteklenmez.
- `--tls insecure` yalnizca `--backend reqwest` ile gecerlidir.
- `--backend impcurl` secildiginde TLS davranisi `curl-impersonate` tarafinda yonetilir.
- Otomatik WAF fallback acikken (`TEFAS_AUTO_WAF=1`) ve `impcurl` kuruluysa, `wreq` secimi runtime'da `impcurl`'e gecirilebilir.

## Combined Output Rule

Birden fazla operasyon verildiğinde sonuç tek bir JSON obje olur; anahtarlar operasyon adlarıdır:

```json
{
  "fonBilgiGetir": [...],
  "getBanners": [...]
}
```

## Kaldırılan Flagler

Aşağıdaki bayraklar artık yoktur (subcommand'larla değiştirildi):

- `--operation` → `query <OP>`
- `--operationOld` → `query --old <OP>`
- `--fetch` → `fetch <URL>` subcommand
- `--parse` → `parse <FILE>` subcommand
- `--payloadjson` → `query --payload <JSON>`
- `--showrequest` → kaldırıldı
- `--writeresponse` → `fetch --output <FILE>`
- `--outputformat` → `query --format json|humanize`
- `--backendnetwork` → `--backend`
- `--backendtls` → `--tls`
- `--impcurl-impersonate` → `--impersonate`
- `--image-output-dir` → `logo --outdir`
- `--image-format` → `logo --format`
- `--testresponse` → kaldırıldı (fixture testler doğrudan `tefas-network`'te yapılır)
- `--takasbank`, `--fundinfo`, `--fonbilgigetir`, `--backend`, `--dboperation` → kaldırıldı

## Related

- [README.md](README.md)
- [quickstart.md](quickstart.md)
- [../api/INPUT_OUTPUT_FIELDS.md](../api/INPUT_OUTPUT_FIELDS.md)
