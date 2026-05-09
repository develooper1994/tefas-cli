//! Service descriptor layer for TEFAS / Takasbank operations.
//!
//! This crate is intentionally network-agnostic: it defines **what** each
//! operation looks like (endpoint path, Referer, default payload, description)
//! without knowing **how** the request is executed.  The caller (e.g.
//! `tefas-cli`) is responsible for wiring in a `NetworkClient`.
//!
//! # Example
//!
//! ```
//! use tefas_api::Operation;
//!
//! let spec = Operation::FonBilgiGetir.spec();
//! println!("POST {}", spec.endpoint);
//! println!("Referer: {}", spec.referer);
//!
//! let payload = Operation::FonBilgiGetir.default_payload();
//! println!("{}", payload);
//! ```

// ── public re-exports ─────────────────────────────────────────────────────────

pub use fund_summary::{FundSummary, normalize_fund_summary_list};
pub use operation::{Operation, OperationOld, OperationSpec};
pub use typed_payloads::{DateRange, FundBilgiGetirPayload, FundFilter, FundProfilDtyGetirPayload};

// ── modules ───────────────────────────────────────────────────────────────────

mod operation {
    use clap::ValueEnum;
    use serde_json::{Value, json};

    /// Endpoint URL path and Referer header for a single Takasbank operation.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OperationSpec {
        /// API endpoint path (e.g. `/api/funds/fonBilgiGetir`) or a full URL
        /// for external services (e.g. `https://api.accessiblee.com/…`).
        pub endpoint: &'static str,
        /// HTTP Referer header value to send with the request.
        pub referer: &'static str,
    }

    /// All 32 current Takasbank/TEFAS JSON API operations.
    ///
    /// Variants intentionally mirror the operation names used in the API so
    /// that mapping from CLI arguments is a straight one-to-one conversion.
    #[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
    pub enum Operation {
        #[value(name = "dagilimSiraliGetirT", alias = "dagilim-sirali-getir-t")]
        DagilimSiraliGetirT,
        #[value(name = "fonBilgiGetir", alias = "fon-bilgi-getir")]
        FonBilgiGetir,
        #[value(
            name = "fonBuyuklukBazliBilgiGetir",
            alias = "fon-buyukluk-bazli-bilgi-getir"
        )]
        FonBuyuklukBazliBilgiGetir,
        #[value(name = "fonDetayGetir", alias = "fon-detay-getir")]
        FonDetayGetir,
        #[value(name = "fonFiyatBilgiGetir", alias = "fon-fiyat-bilgi-getir")]
        FonFiyatBilgiGetir,
        #[value(
            name = "fonGetiriBazliBilgiGetir",
            alias = "fon-getiri-bazli-bilgi-getir"
        )]
        FonGetiriBazliBilgiGetir,
        #[value(name = "fonGnlBlgSiraliGetir", alias = "fon-gnl-blg-sirali-getir")]
        FonGnlBlgSiraliGetir,
        #[value(name = "fonKurucuGetir", alias = "fon-kurucu-getir")]
        FonKurucuGetir,
        #[value(name = "fonProfilDtyGetir", alias = "fon-profil-dty-getir")]
        FonProfilDtyGetir,
        #[value(name = "fonTefasDuyuruGetir", alias = "fon-tefas-duyuru-getir")]
        FonTefasDuyuruGetir,
        #[value(name = "fonTipiGetir", alias = "fon-tipi-getir")]
        FonTipiGetir,
        #[value(name = "fonTurDnmGetiriGetir", alias = "fon-tur-dnm-getiri-getir")]
        FonTurDnmGetiriGetir,
        #[value(name = "fonUnvanAra", alias = "fon-unvan-ara")]
        FonUnvanAra,
        #[value(name = "fonUnvanGetir", alias = "fon-unvan-getir")]
        FonUnvanGetir,
        #[value(
            name = "fonYonetimBazliBilgiGetir",
            alias = "fon-yonetim-bazli-bilgi-getir"
        )]
        FonYonetimBazliBilgiGetir,
        #[value(name = "getBanners", alias = "get-banners")]
        GetBanners,
        #[value(
            name = "getBefasFonTurBazliIslemHacmi",
            alias = "get-befas-fon-tur-bazli-islem-hacmi"
        )]
        GetBefasFonTurBazliIslemHacmi,
        #[value(
            name = "getBefasFonTuruBazindaFonSayisi",
            alias = "get-befas-fon-turu-bazinda-fon-sayisi"
        )]
        GetBefasFonTuruBazindaFonSayisi,
        #[value(
            name = "getBefasToplamIslemHacmi",
            alias = "get-befas-toplam-islem-hacmi"
        )]
        GetBefasToplamIslemHacmi,
        #[value(
            name = "getBefasUyeBazliIslemHacmi",
            alias = "get-befas-uye-bazli-islem-hacmi"
        )]
        GetBefasUyeBazliIslemHacmi,
        #[value(name = "getFplDovizList", alias = "get-fpl-doviz-list")]
        GetFplDovizList,
        #[value(
            name = "getFplFonBazliIslemHacmi",
            alias = "get-fpl-fon-bazli-islem-hacmi"
        )]
        GetFplFonBazliIslemHacmi,
        #[value(name = "getFplFonList", alias = "get-fpl-fon-list")]
        GetFplFonList,
        #[value(
            name = "getFplFonTuruBazindaFonSayisi",
            alias = "get-fpl-fon-turu-bazinda-fon-sayisi"
        )]
        GetFplFonTuruBazindaFonSayisi,
        #[value(name = "getFplHaftaList", alias = "get-fpl-hafta-list")]
        GetFplHaftaList,
        #[value(
            name = "getFplIslemYapanKurumAdet",
            alias = "get-fpl-islem-yapan-kurum-adet"
        )]
        GetFplIslemYapanKurumAdet,
        #[value(name = "getFplMkkStokBakiye", alias = "get-fpl-mkk-stok-bakiye")]
        GetFplMkkStokBakiye,
        #[value(name = "getFplToplamIslemHacmi", alias = "get-fpl-toplam-islem-hacmi")]
        GetFplToplamIslemHacmi,
        #[value(
            name = "getFplUyeBazliIslemHacmi",
            alias = "get-fpl-uye-bazli-islem-hacmi"
        )]
        GetFplUyeBazliIslemHacmi,
        #[value(name = "getLogo", alias = "get-logo")]
        GetLogo,
        #[value(name = "isShowedPopup", alias = "is-showed-popup")]
        IsShowedPopup,
        #[value(name = "validate")]
        Validate,
    }

    impl Operation {
        /// Returns the endpoint path/URL and Referer for this operation.
        pub fn spec(self) -> OperationSpec {
            let (endpoint, referer) = match self {
                Self::DagilimSiraliGetirT => (
                    "/api/funds/dagilimSiraliGetirT",
                    "https://www.tefas.gov.tr/tr/girisim-sermayesi-fonlari?view=portfolioDistribution",
                ),
                Self::FonBilgiGetir => ("/api/funds/fonBilgiGetir", "https://www.tefas.gov.tr/tr"),
                Self::FonBuyuklukBazliBilgiGetir => (
                    "/api/funds/fonBuyuklukBazliBilgiGetir",
                    "https://www.tefas.gov.tr/tr/fon-getirileri?listingTab=size",
                ),
                Self::FonDetayGetir => ("/api/funds/fonDetayGetir", "https://www.tefas.gov.tr/tr"),
                Self::FonFiyatBilgiGetir => (
                    "/api/funds/fonFiyatBilgiGetir",
                    "https://www.tefas.gov.tr/tr",
                ),
                Self::FonGetiriBazliBilgiGetir => (
                    "/api/funds/fonGetiriBazliBilgiGetir",
                    "https://www.tefas.gov.tr/tr/fon-getirileri?listingTab=return",
                ),
                Self::FonGnlBlgSiraliGetir => (
                    "/api/funds/fonGnlBlgSiraliGetir",
                    "https://www.tefas.gov.tr/tr/fon-verileri",
                ),
                Self::FonKurucuGetir => {
                    ("/api/funds/fonKurucuGetir", "https://www.tefas.gov.tr/tr")
                }
                Self::FonProfilDtyGetir => (
                    "/api/funds/fonProfilDtyGetir",
                    "https://www.tefas.gov.tr/tr",
                ),
                Self::FonTefasDuyuruGetir => (
                    "/api/announcements/fonTefasDuyuruGetir",
                    "https://www.tefas.gov.tr/tr",
                ),
                Self::FonTipiGetir => ("/api/funds/fonTipiGetir", "https://www.tefas.gov.tr/tr"),
                Self::FonTurDnmGetiriGetir => (
                    "/api/funds/fonTurDnmGetiriGetir",
                    "https://www.tefas.gov.tr/tr",
                ),
                Self::FonUnvanAra => ("/api/funds/fonUnvanAra", "https://www.tefas.gov.tr/tr"),
                Self::FonUnvanGetir => ("/api/funds/fonUnvanGetir", "https://www.tefas.gov.tr/tr"),
                Self::FonYonetimBazliBilgiGetir => (
                    "/api/funds/fonYonetimBazliBilgiGetir",
                    "https://www.tefas.gov.tr/tr/fon-getirileri?listingTab=management",
                ),
                Self::GetBanners => (
                    "/api/bannerSlider/banner/getBanners",
                    "https://www.tefas.gov.tr/tr",
                ),
                Self::GetBefasFonTurBazliIslemHacmi => (
                    "/api/statistics/befas/getBefasFonTurBazliIslemHacmi",
                    "https://www.tefas.gov.tr/tr",
                ),
                Self::GetBefasFonTuruBazindaFonSayisi => (
                    "/api/statistics/befas/getBefasFonTuruBazindaFonSayisi",
                    "https://www.tefas.gov.tr/tr",
                ),
                Self::GetBefasToplamIslemHacmi => (
                    "/api/statistics/befas/getBefasToplamIslemHacmi",
                    "https://www.tefas.gov.tr/tr",
                ),
                Self::GetBefasUyeBazliIslemHacmi => (
                    "/api/statistics/befas/getBefasUyeBazliIslemHacmi",
                    "https://www.tefas.gov.tr/tr",
                ),
                Self::GetFplDovizList => (
                    "/api/statistics/tefas/getFplDovizList/v2",
                    "https://www.tefas.gov.tr/tr",
                ),
                Self::GetFplFonBazliIslemHacmi => (
                    "/api/statistics/tefas/getFplFonBazliIslemHacmi",
                    "https://www.tefas.gov.tr/tr",
                ),
                Self::GetFplFonList => (
                    "/api/statistics/tefas/getFplFonList",
                    "https://www.tefas.gov.tr/tr",
                ),
                Self::GetFplFonTuruBazindaFonSayisi => (
                    "/api/statistics/tefas/getFplFonTuruBazindaFonSayisi",
                    "https://www.tefas.gov.tr/tr",
                ),
                Self::GetFplHaftaList => (
                    "/api/statistics/tefas/getFplHaftaList",
                    "https://www.tefas.gov.tr/tr",
                ),
                Self::GetFplIslemYapanKurumAdet => (
                    "/api/statistics/tefas/getFplIslemYapanKurumAdet",
                    "https://www.tefas.gov.tr/tr",
                ),
                Self::GetFplMkkStokBakiye => (
                    "/api/statistics/tefas/getFplMkkStokBakiye",
                    "https://www.tefas.gov.tr/tr",
                ),
                Self::GetFplToplamIslemHacmi => (
                    "/api/statistics/tefas/getFplToplamIslemHacmi",
                    "https://www.tefas.gov.tr/tr",
                ),
                Self::GetFplUyeBazliIslemHacmi => (
                    "/api/statistics/tefas/getFplUyeBazliIslemHacmi",
                    "https://www.tefas.gov.tr/tr",
                ),
                Self::GetLogo => ("/api/getLogo/logo/getLogo", "https://www.tefas.gov.tr/tr"),
                Self::IsShowedPopup => (
                    "/api/portal/popup/isShowedPopup",
                    "https://www.tefas.gov.tr/tr",
                ),
                Self::Validate => (
                    "https://api.accessiblee.com/widget/v1/validate",
                    "https://www.tefas.gov.tr/",
                ),
            };
            OperationSpec { endpoint, referer }
        }

        /// Returns the default JSON payload for this operation.
        ///
        /// Callers may mutate the returned `Value` to override individual fields
        /// before sending the request.
        pub fn default_payload(self) -> Value {
            match self {
                Self::DagilimSiraliGetirT => {
                    json!({"fonTipi":"GSYF","fonKodu":null,"aramaMetni":null,"fonTurKod":null,"fonGrubu":null,"sfonTurKod":null,"basTarih":null,"bitTarih":null,"basSira":1,"bitSira":25,"fonTurAciklama":null,"dil":"TR","kurucuKod":null,"sFonTurKod":null,"fonUnvanTip":null})
                }
                Self::FonBilgiGetir => json!({"dil":"TR","fonKodu":""}),
                Self::FonBuyuklukBazliBilgiGetir => {
                    json!({"dil":"TR","fonTipi":"BYF","kurucuKodu":null,"sfonTurKod":null,"fonTurAciklama":null,"islem":null,"fonTurKod":null,"fonGrubu":null,"basTarih":"20260325","bitTarih":"20260425","calismaTipi":1})
                }
                Self::FonDetayGetir => json!({"fonTipi":"BYF","dil":"TR"}),
                Self::FonFiyatBilgiGetir => json!({"fonKodu":"","dil":"TR","periyod":""}),
                Self::FonGetiriBazliBilgiGetir => {
                    json!({"dil":"TR","fonTipi":"YAT","kurucuKodu":null,"sfonTurKod":null,"fonTurAciklama":null,"islem":null,"fonTurKod":null,"fonGrubu":null,"donem1a":null,"donem3a":null,"donem6a":null,"donem1y":null,"donemyb":null,"donem3y":null,"donem5y":null,"basTarih":null,"bitTarih":null,"calismaTipi":null,"getiriOrani":null})
                }
                Self::FonGnlBlgSiraliGetir => {
                    json!({"fonTipi":null,"fonKodu":null,"aramaMetni":null,"fonTurKod":null,"fonGrubu":null,"sfonTurKod":null,"basTarih":null,"bitTarih":null,"basSira":1,"bitSira":25,"fonTurAciklama":null,"dil":"TR","kurucuKod":null})
                }
                Self::FonKurucuGetir => json!({"fonTipi":"EMK","dil":"TR"}),
                Self::FonProfilDtyGetir => {
                    json!({"fonKodu":"","dil":"TR","periyod":"","kf1kod":"","kf2kod":"","kf3kod":"","kf4kod":"","kf5kod":""})
                }
                Self::FonTefasDuyuruGetir => json!({"dil":"TR"}),
                Self::FonTipiGetir => json!({"fonKodu":""}),
                Self::FonTurDnmGetiriGetir => json!({"dil":"TR","donem":13}),
                Self::FonUnvanAra => json!({"dil":"TR","aramaMetni":"","fonTip":null}),
                Self::FonUnvanGetir => json!({"tur":"YAT","dil":"TR"}),
                Self::FonYonetimBazliBilgiGetir => {
                    json!({"dil":"TR","fonTipi":"BYF","kurucuKodu":null,"sfonTurKod":null,"fonTurAciklama":null,"islem":null,"fonTurKod":null,"fonGrubu":null,"basTarih":null,"bitTarih":null,"calismaTipi":null})
                }
                Self::GetBanners => json!({"dil":"TR","cacheTTL":3000}),
                Self::GetBefasFonTurBazliIslemHacmi => {
                    json!({"yil":"","hafta":"","paraBirimi":"TL","dil":"TR"})
                }
                Self::GetBefasFonTuruBazindaFonSayisi => {
                    json!({"yil":null,"ay":null,"hafta":null,"dil":"TR"})
                }
                Self::GetBefasToplamIslemHacmi => {
                    json!({"basYil":"","basAy":"","basHafta":"","bitYil":"","bitAy":"","bitHafta":"","paraBirimi":"TL","dil":"TR"})
                }
                Self::GetBefasUyeBazliIslemHacmi => {
                    json!({"yil":"","hafta":"","paraBirimi":"TL","raporTuru":"DEGER","dil":"TR"})
                }
                Self::GetFplDovizList => json!({"fonKodu":null,"dil":"TR"}),
                Self::GetFplFonBazliIslemHacmi => json!({"yil":"","hafta":"","dil":"TR"}),
                Self::GetFplFonList => json!({}),
                Self::GetFplFonTuruBazindaFonSayisi => {
                    json!({"yil":"","ay":"","hafta":"","dil":"TR"})
                }
                Self::GetFplHaftaList => json!({"yil":"","ay":"","dil":"TR"}),
                Self::GetFplIslemYapanKurumAdet => json!({"yil":"","ay":"","hafta":"","dil":"TR"}),
                Self::GetFplMkkStokBakiye => {
                    json!({"yil":"","ay":"","fonTuru":"","uye":"","paraBirimi":"","dil":"TR"})
                }
                Self::GetFplToplamIslemHacmi => {
                    json!({"basYil":"","basAy":"","basHafta":"","bitYil":"","bitAy":"","bitHafta":"","paraBirimi":"TL","dil":"TR"})
                }
                Self::GetFplUyeBazliIslemHacmi => json!({"yil":"","hafta":"","dil":"TR"}),
                Self::GetLogo => json!({"memberCodes":[]}),
                Self::IsShowedPopup => json!({}),
                Self::Validate => {
                    json!({"accountId":"","originDomain":"www.tefas.gov.tr","fullPageUrl":"https://www.tefas.gov.tr/tr","referrer":""})
                }
            }
        }

        /// Returns a typed payload when a struct model exists for this operation.
        ///
        /// This keeps backward compatibility with `default_payload()` while
        /// allowing compile-time field safety for common requests.
        pub fn default_payload_typed(self) -> Option<Value> {
            match self {
                Self::FonBilgiGetir => Some(
                    serde_json::to_value(crate::typed_payloads::FundBilgiGetirPayload::default())
                        .expect("typed payload must serialize"),
                ),
                Self::FonProfilDtyGetir => Some(
                    serde_json::to_value(
                        crate::typed_payloads::FundProfilDtyGetirPayload::default(),
                    )
                    .expect("typed payload must serialize"),
                ),
                Self::FonGetiriBazliBilgiGetir => Some(
                    serde_json::to_value(crate::typed_payloads::FundFilter::default())
                        .expect("typed payload must serialize"),
                ),
                _ => None,
            }
        }

        /// Returns a short English description for `--help` displays.
        pub fn description(self) -> &'static str {
            match self {
                Self::DagilimSiraliGetirT => "Portfolio distribution list by fund type",
                Self::FonBilgiGetir => "General fund list/information",
                Self::FonBuyuklukBazliBilgiGetir => "Fund sizes filtered by criteria",
                Self::FonDetayGetir => "Fund detail metadata",
                Self::FonFiyatBilgiGetir => "Fund price/time series info",
                Self::FonGetiriBazliBilgiGetir => "Fund returns filtered by periods",
                Self::FonGnlBlgSiraliGetir => "Sorted general fund information",
                Self::FonKurucuGetir => "Fund founders/providers list",
                Self::FonProfilDtyGetir => "Detailed profile and portfolio breakdown",
                Self::FonTefasDuyuruGetir => "TEFAS announcements",
                Self::FonTipiGetir => "Fund type list",
                Self::FonTurDnmGetiriGetir => "Periodic return by fund class",
                Self::FonUnvanAra => "Search funds by name/title",
                Self::FonUnvanGetir => "Fund title list by type",
                Self::FonYonetimBazliBilgiGetir => "Management-based fund metrics",
                Self::GetBanners => "Homepage banner payload",
                Self::GetBefasFonTurBazliIslemHacmi => "BEFAS volume by fund type",
                Self::GetBefasFonTuruBazindaFonSayisi => "BEFAS fund counts by type",
                Self::GetBefasToplamIslemHacmi => "BEFAS total trading volume",
                Self::GetBefasUyeBazliIslemHacmi => "BEFAS member-based volume",
                Self::GetFplDovizList => "FPL currency list",
                Self::GetFplFonBazliIslemHacmi => "FPL volume by fund",
                Self::GetFplFonList => "FPL fund list",
                Self::GetFplFonTuruBazindaFonSayisi => "FPL fund counts by type",
                Self::GetFplHaftaList => "FPL week list",
                Self::GetFplIslemYapanKurumAdet => "FPL active institution count",
                Self::GetFplMkkStokBakiye => "FPL MKK stock balances",
                Self::GetFplToplamIslemHacmi => "FPL total trading volume",
                Self::GetFplUyeBazliIslemHacmi => "FPL member-based volume",
                Self::GetLogo => "Institution logos (base64 images)",
                Self::IsShowedPopup => "Popup visibility state",
                Self::Validate => "Accessiblee widget validation",
            }
        }
    }

    /// All 8 legacy ASMX / DB operations.
    #[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
    pub enum OperationOld {
        #[value(name = "bindChartData", alias = "BindChartData")]
        BindChartData,
        #[value(
            name = "bindComparisonFundReturns",
            alias = "BindComparisonFundReturns"
        )]
        BindComparisonFundReturns,
        #[value(name = "bindComparisonFundSizes", alias = "BindComparisonFundSizes")]
        BindComparisonFundSizes,
        #[value(
            name = "bindComparisonManagementFees",
            alias = "BindComparisonManagementFees"
        )]
        BindComparisonManagementFees,
        #[value(name = "bindHistoryAllocation", alias = "BindHistoryAllocation")]
        BindHistoryAllocation,
        #[value(name = "bindHistoryInfo", alias = "BindHistoryInfo")]
        BindHistoryInfo,
        #[value(name = "getAllFundAnalyzeData", alias = "GetAllFundAnalyzeData")]
        GetAllFundAnalyzeData,
        #[value(name = "getAllFunds", alias = "GetAllFunds")]
        GetAllFunds,
    }

    impl OperationOld {
        /// Returns the endpoint path and Referer for this legacy operation.
        pub fn spec(self) -> OperationSpec {
            let (endpoint, referer) = match self {
                Self::BindChartData => (
                    "/Default.aspx/BindChartData",
                    "https://www.tefas.gov.tr/Default.aspx",
                ),
                Self::BindComparisonFundReturns => (
                    "/api/DB/BindComparisonFundReturns",
                    "https://www.tefas.gov.tr/FonKarsilastirma.aspx",
                ),
                Self::BindComparisonFundSizes => (
                    "/api/DB/BindComparisonFundSizes",
                    "https://www.tefas.gov.tr/FonKarsilastirma.aspx",
                ),
                Self::BindComparisonManagementFees => (
                    "/api/DB/BindComparisonManagementFees",
                    "https://www.tefas.gov.tr/FonKarsilastirma.aspx",
                ),
                Self::BindHistoryAllocation => (
                    "/api/DB/BindHistoryAllocation",
                    "https://www.tefas.gov.tr/TarihselVeriler.aspx",
                ),
                Self::BindHistoryInfo => (
                    "/api/DB/BindHistoryInfo",
                    "https://www.tefas.gov.tr/TarihselVeriler.aspx",
                ),
                Self::GetAllFundAnalyzeData => (
                    "/api/DB/GetAllFundAnalyzeData",
                    "https://www.tefas.gov.tr/FonAnaliz.aspx",
                ),
                Self::GetAllFunds => (
                    "/Service.asmx/GetAllFunds",
                    "https://www.tefas.gov.tr/FonAnaliz.aspx?FonKod=TLY",
                ),
            };
            OperationSpec { endpoint, referer }
        }

        /// Returns the default JSON payload for this legacy operation.
        pub fn default_payload(self) -> Value {
            match self {
                Self::BindChartData => json!({"period": 1}),
                Self::BindComparisonFundReturns => {
                    json!({"calismatipi":2,"fontip":"YAT","sfontur":"","kurucukod":"","fongrup":"","bastarih":"Başlangıç","bittarih":"Bitiş","fonturkod":"","fonunvantip":"","strperiod":"1,1,1,1,1,1,1","islemdurum":"1"})
                }
                Self::BindComparisonFundSizes => {
                    json!({"calismatipi":2,"fontip":"YAT","sfontur":"","kurucukod":"","fongrup":"","bastarih":"01.01.2024","bittarih":"31.01.2024","fonturkod":"","fonunvantip":"","strperiod":"1,1,1,1,1,1,1","islemdurum":"1"})
                }
                Self::BindComparisonManagementFees => {
                    json!({"calismatipi":2,"fontip":"YAT","sfontur":"","kurucukod":"","fongrup":"","bastarih":"Başlangıç","bittarih":"Bitiş","fonturkod":"","fonunvantip":"","strperiod":"1,1,1,1,1,1,1","islemdurum":"1"})
                }
                Self::BindHistoryAllocation => {
                    json!({"fontip":"ALL","bastarih":"2023-01-01","bittarih":"2023-01-31","fonkod":"AFA"})
                }
                Self::BindHistoryInfo => {
                    json!({"fontip":"ALL","bastarih":"2023-01-01","bittarih":"2023-01-31","fonkod":"AFA"})
                }
                Self::GetAllFundAnalyzeData => {
                    json!({"fonTip":"YAT","fonKod":"TLY","bastarih":"01.01.2026","bittarih":"31.01.2026"})
                }
                Self::GetAllFunds => json!({"prefixText":"","count":20,"contextKey":null}),
            }
        }

        /// Returns a short English description for `--help` displays.
        pub fn description(self) -> &'static str {
            match self {
                Self::BindChartData => "Legacy chart data binding endpoint",
                Self::BindComparisonFundReturns => "Legacy fund return comparison",
                Self::BindComparisonFundSizes => "Legacy fund size comparison",
                Self::BindComparisonManagementFees => "Legacy management fee comparison",
                Self::BindHistoryAllocation => "Legacy history allocation endpoint",
                Self::BindHistoryInfo => "Legacy history info endpoint",
                Self::GetAllFundAnalyzeData => "Legacy all-fund analysis data",
                Self::GetAllFunds => "Legacy ASMX get-all-funds service",
            }
        }
    }
}

mod fund_summary {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct FundSummary {
        pub fund_name: Option<String>,
        pub fund_code: Option<String>,
    }

    impl FundSummary {
        pub fn human_line(&self) -> String {
            let name = self.fund_name.as_deref().unwrap_or("");
            let code = self.fund_code.as_deref().unwrap_or("");
            format!("{} ({})", name, code)
        }
    }

    fn to_summary(item: &Value) -> FundSummary {
        let fund_name = item
            .get("fonUnvan")
            .or_else(|| item.get("unvan"))
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let fund_code = item
            .get("fonKodu")
            .or_else(|| item.get("fonKod"))
            .and_then(Value::as_str)
            .map(ToString::to_string);

        FundSummary {
            fund_name,
            fund_code,
        }
    }

    pub fn normalize_fund_summary_list(raw: &Value) -> Vec<FundSummary> {
        let source = raw
            .get("resultList")
            .or_else(|| raw.get("data"))
            .unwrap_or(raw);

        match source {
            Value::Array(items) => items.iter().map(to_summary).collect(),
            Value::Null => Vec::new(),
            other => vec![to_summary(other)],
        }
    }
}

mod typed_payloads {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct DateRange {
        #[serde(rename = "basTarih")]
        pub bas_tarih: Option<String>,
        #[serde(rename = "bitTarih")]
        pub bit_tarih: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FundBilgiGetirPayload {
        #[serde(rename = "dil")]
        pub dil: String,
        #[serde(rename = "fonKodu")]
        pub fon_kodu: String,
    }

    impl Default for FundBilgiGetirPayload {
        fn default() -> Self {
            Self {
                dil: "TR".to_string(),
                fon_kodu: String::new(),
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FundProfilDtyGetirPayload {
        #[serde(rename = "fonKodu")]
        pub fon_kodu: String,
        #[serde(rename = "dil")]
        pub dil: String,
        #[serde(rename = "periyod")]
        pub periyod: String,
        #[serde(rename = "kf1kod")]
        pub kf1_kod: String,
        #[serde(rename = "kf2kod")]
        pub kf2_kod: String,
        #[serde(rename = "kf3kod")]
        pub kf3_kod: String,
        #[serde(rename = "kf4kod")]
        pub kf4_kod: String,
        #[serde(rename = "kf5kod")]
        pub kf5_kod: String,
    }

    impl Default for FundProfilDtyGetirPayload {
        fn default() -> Self {
            Self {
                fon_kodu: String::new(),
                dil: "TR".to_string(),
                periyod: String::new(),
                kf1_kod: String::new(),
                kf2_kod: String::new(),
                kf3_kod: String::new(),
                kf4_kod: String::new(),
                kf5_kod: String::new(),
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FundFilter {
        #[serde(rename = "dil")]
        pub dil: String,
        #[serde(rename = "fonTipi")]
        pub fon_tipi: String,
        #[serde(rename = "kurucuKodu")]
        pub kurucu_kodu: Option<String>,
        #[serde(rename = "sfonTurKod")]
        pub sfon_tur_kod: Option<String>,
        #[serde(rename = "fonTurAciklama")]
        pub fon_tur_aciklama: Option<String>,
        #[serde(rename = "islem")]
        pub islem: Option<String>,
        #[serde(rename = "fonTurKod")]
        pub fon_tur_kod: Option<String>,
        #[serde(rename = "fonGrubu")]
        pub fon_grubu: Option<String>,
        #[serde(rename = "donem1a")]
        pub donem_1a: Option<String>,
        #[serde(rename = "donem3a")]
        pub donem_3a: Option<String>,
        #[serde(rename = "donem6a")]
        pub donem_6a: Option<String>,
        #[serde(rename = "donem1y")]
        pub donem_1y: Option<String>,
        #[serde(rename = "donemyb")]
        pub donem_yb: Option<String>,
        #[serde(rename = "donem3y")]
        pub donem_3y: Option<String>,
        #[serde(rename = "donem5y")]
        pub donem_5y: Option<String>,
        #[serde(flatten)]
        pub date_range: DateRange,
        #[serde(rename = "calismaTipi")]
        pub calisma_tipi: Option<i32>,
        #[serde(rename = "getiriOrani")]
        pub getiri_orani: Option<f64>,
    }

    impl Default for FundFilter {
        fn default() -> Self {
            Self {
                dil: "TR".to_string(),
                fon_tipi: "YAT".to_string(),
                kurucu_kodu: None,
                sfon_tur_kod: None,
                fon_tur_aciklama: None,
                islem: None,
                fon_tur_kod: None,
                fon_grubu: None,
                donem_1a: None,
                donem_3a: None,
                donem_6a: None,
                donem_1y: None,
                donem_yb: None,
                donem_3y: None,
                donem_5y: None,
                date_range: DateRange::default(),
                calisma_tipi: None,
                getiri_orani: None,
            }
        }
    }
}

#[cfg(test)]
mod tests;
