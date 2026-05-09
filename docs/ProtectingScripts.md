# Protecting scripts: `shc` and alternatives

This document explains options to produce protected versions of the bash scripts in this repo.

## Available approaches

- shc (C wrapper): compiles a shell script into a binary by embedding the script into a C program. Good for distributing a binary; requires `shc` to be installed (Debian/Ubuntu: `apt install shc`).

- bash-obfuscate (npm): obfuscates a bash script source into a harder-to-read form. Install with `npm i -g @leovoel/bash-obfuscate` or run via `npx bash-obfuscate`.

- upx: not an obfuscator, but compresses native binaries produced by `shc` to reduce size and slightly hinder reverse-engineering: `apt install upx`.

## Usage in this repo

Koruma akışı xtask altında sunulur: `cargo xtask tools protect`.

### Examples

- Detect available tools:

  cargo xtask tools protect --list

- Compile with `shc` (preferred if you want a binary):

  cargo xtask tools protect -i path/to/script.sh -t shc -o bin/script

- Compile with `bunster` (if installed):

  cargo xtask tools protect -i path/to/script.sh -t bunster -o bin/script

- Obfuscate with `bash-obfuscate` (source remains a shell script):

  cargo xtask tools protect -i path/to/script.sh -t bash-obfuscate -o obf/script.sh

### Notes and caveats

- `shc`-produced binaries still depend on libc and may show system-specific behavior; test on target platforms.
- Obfuscation is not a security measure; it only raises the bar for casual inspection.
- We recommend keeping `--strip` off by default; always preserve original sources in your repo.

### Installation hints

- Debian/Ubuntu: `sudo apt install shc upx`
- Node-based obfuscator: `npm i -g @leovoel/bash-obfuscate` or use `npx bash-obfuscate` without global install.
