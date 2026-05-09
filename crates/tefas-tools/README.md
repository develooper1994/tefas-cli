# tefas-tools

Geliştirici araçları — çok-strateji bağımlılık kurulum yardımcı programı.

**Paket adı:** `tefas-tools` &nbsp;·&nbsp; **Dizin:** `crates/tefas-tools`

---

## Sorumluluk

`install_with_fallback` fonksiyonu: bir araç zaten kuruluysa atlar, değilse platform paket yöneticilerini sırayla dener:

```
apt-get  →  dnf  →  yum  →  pacman  →  apk

macOS: brew

Windows: winget  →  scoop  →  choco

Paket yöneticileri başarısız olursa:

1. Prebuilt artifact adımı: dosya Rust-native HTTP (`reqwest`) ile indirilir ve hedefe kopyalanır.
2. Source build adımı: kaynak çekilip build/install komutları çalıştırılır.
```

---

## Kullanım

```rust
use tefas_tools::{InstallOptions, install_with_fallback};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    install_with_fallback(InstallOptions {
        name: "cmake".to_string(),
        check_cmd: "cmake".to_string(),
        apt: vec!["cmake".to_string()],
        dnf: vec!["cmake".to_string()],
        yum: vec!["cmake".to_string()],
        pacman: vec!["cmake".to_string()],
        apk: vec!["cmake".to_string()],
        brew: vec!["cmake".to_string()],
        winget: vec!["Kitware.CMake".to_string()],
        scoop: vec!["cmake".to_string()],
        choco: vec!["cmake".to_string()],
        ..InstallOptions::default()
    }).await?;
    Ok(())
}
```

---

## İlgili

- `cargo xtask tools install-fallback [OPTIONS]` — bu crate'in xtask tabanli komut arayuzu
