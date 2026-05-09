# Documentation Index

Tüm teknik belgeler bu dizin altındadır.

---

## 📐 Architecture

| Belge | İçerik |
|-------|--------|
| [architecture.md](architecture.md) | Crate bağımlılık grafiği, sorumluluk sınırları, domain + use-case katman ayrımı |
| [migration-map.md](migration-map.md) | Legacy shell akışları → yeni CLI flag eşlemesi |
| [parser-hotspot-plan.md](parser-hotspot-plan.md) | Parser regex/string hotspot'ları için ölçüm odaklı optimizasyon planı |
| [parser-rsc-legacy-refactor.md](parser-rsc-legacy-refactor.md) | Fund page parser içinde RSC ve legacy akışlarını ayıran refactor taslağı |

---

## ⚡ Operations (CLI Kullanımı)

| Belge | İçerik |
|-------|--------|
| [operations/quickstart.md](operations/quickstart.md) | Hızlı başlangıç örnekleri |
| [operations/reference.md](operations/reference.md) | Tüm flag ve davranış referansı |
| [operations/README.md](operations/README.md) | Operations bölümü giriş noktası |
| [operations/low_level_http.md](operations/low_level_http.md) | Low-level HTTP kontrol araştırması (libcurl, raw socket) |
| [operations/profiling.md](operations/profiling.md) | Criterion + samply ile darboğaz analizi ve iteratif optimizasyon döngüsü |
| [usage.md](usage.md) | Tools namespace altındaki script koruma ve yardımcı kullanım notları |
| [ProtectingScripts.md](ProtectingScripts.md) | `cargo xtask tools protect` ile script koruma/obfuscation seçenekleri |

---

## 🗃️ Takasbank API

| Belge | İçerik |
|-------|--------|
| [api/INPUT_OUTPUT_FIELDS.md](api/INPUT_OUTPUT_FIELDS.md) | Alan bazlı girdi/çıktı notları |

---

## Navigasyon

- Projeye yeni başlıyorsanız → [operations/quickstart.md](operations/quickstart.md)
- Mimariyi anlamak istiyorsanız → [architecture.md](architecture.md)
- Belirli bir flag arıyorsanız → [operations/reference.md](operations/reference.md)
- Network backend seçimi → [operations/low_level_http.md](operations/low_level_http.md)
- Workspace kökü → [../README.md](../README.md)
- Shared crate dokümantasyonu → [shared/README.md](shared/README.md)
- Shared HTTP client → [shared/crates/http-client/README.md](shared/crates/http-client/README.md)
- Shared HTTP config → [shared/crates/http-config/README.md](shared/crates/http-config/README.md)
- Shared image util → [shared/crates/image-util/README.md](shared/crates/image-util/README.md)
