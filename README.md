<p align="center">
  <img src="assets/icon-512.png" width="128" alt="Randomon icon">
</p>

<p align="center">
  <h1 align="center">Randomon</h1>
  <p align="center">A modern, cross-platform Pokémon randomizer, fully compatible with Universal Pokemon Randomizer ZX settings and seeds.</p>
</p>

<p align="center">
  <a href="https://github.com/sponsors/castdrian">
    <img src="https://img.shields.io/badge/sponsor-30363D?style=for-the-badge&logo=GitHub-Sponsors&logoColor=#white" alt="Sponsor">
  </a>
  <a href="https://ko-fi.com/castdrian">
    <img src="https://img.shields.io/badge/Ko--fi-F16061?style=for-the-badge&logo=ko-fi&logoColor=white" alt="Ko-FI">
  </a>
  <a href="https://discord.gg/NdaBaxFKnn">
    <img src="https://img.shields.io/badge/Discord-5865F2?style=for-the-badge&logo=discord&logoColor=white" alt="Discord">
  </a>
  <a href="https://github.com/castdrian/randomon">
    <img src="https://img.shields.io/badge/GitHub-100000?style=for-the-badge&logo=github&logoColor=white" alt="GitHub">
  </a>
</p>

<p align="center">
  <a href="https://github.com/castdrian/randomon/actions/workflows/ci.yml">
    <img src="https://github.com/castdrian/randomon/actions/workflows/ci.yml/badge.svg" alt="CI">
  </a>
  <a href="https://github.com/castdrian/randomon/releases/">
    <img src="https://img.shields.io/github/release/castdrian/randomon?include_prereleases=&sort=semver&color=blue" alt="GitHub release">
  </a>
  <a href="#license">
    <img src="https://img.shields.io/badge/License-GPLv3-blue" alt="License">
  </a>
</p>

> [!WARNING]
> Randomon is under active development. The app shell and the file-format layers are in place, and the randomization engine is being ported generation by generation. Until a generation reaches parity, keep using UPR ZX for it.

## Features

- **Same results as UPR ZX 4.6.1.** The same ROM, settings string and seed produce a byte-identical ROM, the same log and the same check value.
- **Reads every UPR file:** settings strings from any UPR version, `.rnqs` quick settings, `.rndp` presets and `customnames.rncn`.
- **Every UPR game, Gen 1 to 7:** GB, GBC, GBA, NDS and 3DS, including 3DS game updates and LayeredFS output.
- **A native app** for macOS, Windows and Linux. No Java needed.
- **A command-line tool** that accepts UPR's CLI flags, plus `--seed` and `--settings-string`.

## Download

Grab the latest build from the [releases page](https://github.com/castdrian/randomon/releases).

| Platform | Files |
|---|---|
| macOS (Apple silicon and Intel) | `Randomon-<version>-macos-universal.dmg` (signed and notarized) |
| Windows (x64 and ARM64) | `-setup.exe` installer or `-portable.exe` |
| Linux (x86_64 and aarch64) | `.AppImage` or `.tar.gz` |

## Command line

```bash
randomon-cli -i "Pokemon Alpha Sapphire.3ds" -u "Update v1.4.cia" -o out -d -s settings.rnqs --seed 123456789 -l
```

## Building

```bash
git clone --recursive https://github.com/castdrian/randomon
```

```bash
cargo run -p randomon
```

The `upstream` submodule pins the UPR ZX source that Randomon is checked against. Parity tools and test vectors live in `tools/`. See [PLAN.md](PLAN.md) for the architecture and roadmap.

## Credits

Randomon is a Rust port of [Universal Pokemon Randomizer ZX](https://github.com/Ajarmar/universal-pokemon-randomizer-zx) by Ajarmar, with significant contributions from darkeye and cleartonic. That project is based on the Universal Pokemon Randomizer by Dabomstew. Brilliant Diamond and Shining Pearl support is derived from [BDSP-Randomizers](https://github.com/Ai0796/BDSP-Randomizers) by Aldo796, Copycat, SanGawku, XLuma and Red (MIT), including its exefs starter offsets and the species name, category, body size and run speed tables in `crates/randomon_bdsp/assets`. Pokémon and all related names are trademarks of Nintendo, Creatures and Game Freak. Randomon ships no game data. Bring your own legally dumped games.

## License

Randomon is licensed under the [GNU General Public License v3.0 or later](LICENSE), the same license as UPR ZX.
