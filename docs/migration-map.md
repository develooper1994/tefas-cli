# Migration Map: Legacy -> tefas

Bu doküman, legacy shell akışlarının `tefas` içindeki subcommand tabanlı CLI modlarına taşındığını özetler.

## Mode Mapping

| Legacy flow | New mode |
|---|---|
| FundPage parser akışları | `tefas parse <FILE>...` veya `tefas fundpage <CODE>...` |
| URL fetch yardımcıları | `tefas fetch <URL>...` |
| Stats endpoint çağrıları | `tefas query <op>` |
| Native/legacy operation çağrıları | `tefas query <op>` veya `tefas query --old <op>` |

## Operation Coverage

`query` ve `query --old` altında legacy script ailesindeki operasyonların tamamı taşınmıştır.
Operasyon listesini görmek için:

```bash
cargo run -q -p tefas-cli -- query --list
```

## Cutover Gates

- Parser fixture parity testi geçmeli
- `cargo check --workspace` geçmeli
- `cargo test --workspace` geçmeli
