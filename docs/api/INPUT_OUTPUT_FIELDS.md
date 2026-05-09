# Tefas Takasbank — Input / Output Alan Listesi

Not: Çıktı alanları repository'deki `datasets` örneklerinden alınmıştır; scriptler gerçek çalıştırmada daha fazla veya daha az alan döndürebilir. Bu nedenle `datasets` tam referans olarak kullanılmamalıdır — sadece örnek amaçlıdır.

Her bölüm: Script adı / Girdi alanları / Çıktı alanları (örnek).

---

## fonGetiriBazliBilgiGetir
- Girdi alanları: dil, fonTipi, kurucuKodu, sfonTurKod, fonTurAciklama, islem, fonTurKod, fonGrubu, donemGetiri1a, donemGetiri3a, donemGetiri6a, donemGetiri1y, donemGetiriyb, donemGetiri3y, donemGetiri5y, basTarih, bitTarih, calismaTipi, getiriOrani
- Çıktı alanları (örnek): errorCode, errorMessage, resultList[].fonKodu, resultList[].fonUnvan, resultList[].fonTurAciklama, resultList[].tefasDurum, resultList[].getiri1a, resultList[].getiri3a, resultList[].getiri6a, resultList[].getiri1y, resultList[].getiriyb, resultList[].getiri3y, resultList[].getiri5y, resultList[].getiriOrani, resultList[].riskDegeri

## fonGnlBlgSiraliGetir
## fonGnlBlgSiraliGetir
- Girdi alanları: fonTipi, fonKodu, aramaMetni, fonTurKod, fonGrubu, sfonTurKod, basTarih, bitTarih, basSira, bitSira, fonTurAciklama, kurucuKod, dil
- Çıktı alanları (örnek): errorCode, errorMessage, resultList[].fonKodu, resultList[].fonUnvan, resultList[].tarih, resultList[].fiyat, resultList[].tedPaySayisi, resultList[].kisiSayisi, resultList[].portfoyBuyukluk, resultList[].rn, resultList[].borsaBultenFiyat, toplamSayi, toplamSayfa

## fonKurucuGetir
- Girdi alanları: fonTipi, dil
- Çıktı alanları (örnek): resultList[].kurucuKodu, resultList[].kurucuUnvan, resultList[].fonTipi

## fonProfilDtyGetir
- Girdi alanları: dil, fonKodu, periyod, kf1kod, kf2kod, kf3kod, kf4kod, kf5kod
- Çıktı alanları (örnek): errorCode, errorMessage, resultList[].fonKodu, resultList[].fonUnvan, resultList[].fonTuru, resultList[].fonTurGetiri

## fonTefasDuyuruGetir
- Girdi alanları: dil
- Çıktı alanları (örnek): [ { duyuruBaslik, siraNo, duyuruDetay } ]

## fonTipiGetir
- Girdi alanları: fonKodu
- Çıktı alanları (örnek): errorCode, errorMessage, fonTipi

## fonUnvanAra
- Girdi alanları: dil, aramaMetni, fonTipi (script option). Script accepts `-T`, `--fontipi` and also `--fundtype` (canonical alias). Dataset example uses JSON key `fonTip`. Allowed values: YAT|EMK|BYF|ALL — `ALL` = no filter; when `ALL` is used the payload emits `null` for `fonTip` and the Referer omits `fundType`.
- Dataset fixture: datasets/api/fonUnvanAra/input.req.txt
- Dataset output: datasets/api/fonUnvanAra/output.body
- Çıktı alanları (örnek): resultList[]  (dataset örneğinde boş dizi; örnek alan yok)

## fonUnvanGetir
- Girdi alanları: fundType (CLI: -T, --fundtype; payload key: `tur`; allowed values: YAT|EMK|BYF; default: YAT), dil
- Çıktı alanları (örnek): resultList[].tanim

## fonYonetimBazliBilgiGetir
- Girdi alanları: dil, fonTipi, kurucuKodu, sfonTurKod, fonTurAciklama, islem, fonTurKod, fonGrubu
- Çıktı alanları (örnek): errorCode, errorMessage, resultList[].fonKodu, resultList[].fonUnvan, resultList[].fonTurAciklama, resultList[].tefasDurum, resultList[].fonTurKod, resultList[].kurucuKod, resultList[].altbaslik1, resultList[].uygulananYu1Y, resultList[].fonIcTuzukYu1G, resultList[].yillikGetiri, resultList[].fonTopGiderKesoran

## getBanners
## getBanners
- Girdi alanları: dil, cacheTTL
- Dataset fixture: datasets/api/getBanners/output.body
- Çıktı alanları (örnek): base64Image, base64LogoImage, base64MobileImage, buttonText, description, endDate, imageAlt, startDate, title, url

## getBefasFonTurBazliIslemHacmi
- Girdi alanları: yil, hafta, paraBirimi, dil
- Çıktı alanları (örnek): data[].fonTuru, data[].islemHacmi, data[].hacimDagilimOran

## getBefasFonTuruBazindaFonSayisi
- Girdi alanları: yil (required), ay (optional; zero-padded as `MM` when provided), hafta (optional)
- Notes: `--ay` and `--hafta` are mutually exclusive; `--yil` is required for real requests (test mode `TEFAS_TEST_RESPONSE` bypasses this).
- Dataset fixture: datasets/api/getBefasFonTuruBazindaFonSayisi/input.req.txt
- Dataset fixture: datasets/api/getBefasFonTuruBazindaFonSayisi/input.req.txt
- Dataset output: datasets/api/getBefasFonTuruBazindaFonSayisi/output.body
- Çıktı alanları (örnek): data[].fonTuru, data[].adet

## getBefasToplamIslemHacmi
- Girdi alanları: basYil, basAy, basHafta, bitYil, bitAy, bitHafta, paraBirimi, dil
- Çıktı alanları (örnek): data[].donem, data[].toplamIslemHacmi, data[].gunlukOrtalamaHacim, data[].netTakasTutar

## getBefasUyeBazliIslemHacmi
- Girdi alanları: yil (required), hafta (optional), paraBirimi, raporTuru, dil
- Dataset fixture: datasets/api/getBefasUyeBazliIslemHacmi/input.req.txt
- Dataset output: datasets/api/getBefasUyeBazliIslemHacmi/output.body
- Çıktı alanları (örnek): data[].uyeKod, data[].kurumUnvan, data[].islemHacmi, data[].hacimOran

## getFplDovizList
- Girdi alanları: fonKodu, dil
- Çıktı alanları (örnek): data[].dovizKod, data[].dovizAd

## getFplFonList
- Girdi alanları: (örnek: none / payload `{}`)
- Dataset output: datasets/api/getFplFonList/output.body
- Çıktı alanları (örnek): (dataset örneği boş — örnek alan yok)

## getFplMkkStokBakiye
- Girdi alanları: yil, ay, fonTuru, uye, paraBirimi, dil
- Çıktı alanları (örnek): data[].uye, data[].uyeUnvan, data[].fonTur, data[].fonAdet, data[].portfoyDegeri

## getFplToplamIslemHacmi
- Girdi alanları: basYil, basAy, basHafta, bitYil, bitAy, bitHafta, paraBirimi, dil
- Çıktı alanları (örnek): data[].donem, data[].toplamIslemHacmi, data[].gunlukOrtalamaHacim, data[].netTakasTutar

## fonTurDnmGetiriGetir
- Girdi alanları: dil, donem
- Çıktı alanları (örnek): errorCode, errorMessage, resultList[].fonTurAciklama, resultList[].topFonSayisi, resultList[].pazarBuyukluk, resultList[].fonTurKod, resultList[].fonDonemGetiri, resultList[].obje1List

## getFplFonBazliIslemHacmi
- Girdi alanları: yil, hafta, dil
- Çıktı alanları (örnek): data[].fonKod, data[].fonUnvan, data[].islemHacmi, data[].hacimDagilimOran  (top-level: data, error, errorCode, errorMessage)

## getFplUyeBazliIslemHacmi
- Girdi alanları: yil, hafta, dil
- Çıktı alanları (örnek): data[].uyeKod, data[].kurumUnvan, data[].islemHacmi, data[].hacimOran  (top-level: data, error, errorCode, errorMessage)

## getFplFonTuruBazindaFonSayisi
- Girdi alanları: yil, ay, hafta, dil  (not: `--ay` ve `--hafta` kullanılıyorsa `--yil` gereklidir)
- Çıktı alanları (örnek): errorCode, errorMessage, data[].fonTuru, data[].adet  (top-level: data, error, errorCode, errorMessage)

## getFplHaftaList
- Girdi alanları: yil, ay, dil
- Çıktı alanları (örnek): [ { hafta, haftaDonem } ]

## getLogo
- Girdi alanları: "memberCodes":[]
- Çıktı alanları (örnek): [{base64LogoImage, title, description, url, memberCode, startDate, endDate}[]]

## Not: Denenen ama kaydedilmeyen scriptler
## getFplIslemYapanKurumAdet
- Girdi alanları: yil, ay, hafta, dil  (not: `--yil` gerekli; `--ay` ve `--hafta` bir arada kullanılmamalıdır)
- Çıktı alanları (örnek): errorCode, errorMessage, data[].uyeTipi, data[].uyeSayisi  (top-level: data, error, errorCode, errorMessage)

## Not: Denenen ama kaydedilmeyen scriptler
 - (none remain in this category)

---

Not: Bu dosya sadece alan isimlerini hızlıca görmek için hazırlanmıştır. Daha kapsamlı veya farklı format (CSV/JSON) isterseniz söyleyin, güncelleyeyim.
