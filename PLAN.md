# Randomon: plan

A modern, cross-platform Rust rewrite of [Universal Pokemon Randomizer ZX](https://github.com/Ajarmar/universal-pokemon-randomizer-zx) (UPR ZX). The repo and the app are both called **randomon**, at `castdrian/randomon`.

## 1. Goals

- **Full compatibility with UPR ZX 4.6.1** (internal version `322`, upstream commit `7f00eb8`). The same ROM, settings and seed must produce the same output as UPR ZX.
- **Every game UPR ZX supports**, Gen 1 to 7: GB/GBC, GBA, NDS and 3DS, including 3DS game updates and LayeredFS output.
- **A polished native GUI** on macOS, Windows and Linux, plus a CLI that accepts the same flags as UPR's CLI.
- **The same project setup as your other repos:** sponsor badges, `FUNDING.yml`, a donate button in the app, and pinned GitHub Actions with releases built from tags.

**Not goals for 1.0:** new randomization features, new games, or anything that would make output differ from UPR. New features can come after parity, behind a "Randomon-only" settings namespace so parity mode stays pure.

## 2. Why Rust

- Your Impactor reference uses **iced 0.14 + wgpu**. That stack is GPU-rendered, themeable, and builds native binaries for all three OSes without a webview.
- Rust can match Java's integer and float behavior exactly (wrapping `i32` math, IEEE doubles). Bit-exact parity depends on that.
- The output is a single binary with no JVM, which is the biggest usability complaint about UPR.

Go would work for the engine, but its GUI options are weaker, and you asked for an Impactor-style UI.

## 3. What "fully compatible" means

Compatibility is the hard part of this project, so it's defined precisely here.

### 3.1 File formats (read and write)

| Artifact | Format (from UPR source) |
|---|---|
| Settings string | Base64 of the settings bytes (51 bytes of flags) + ROM name + CRC32 + checksum of `customnames.rncn`. The GUI and log show it with the 3-digit version in front (`322…`), and older prefixes go through the settings updater. |
| `.rnqs` quick settings | Big-endian `i32 VERSION`, `i32 len`, then the settings string as UTF-8 |
| `.rndp` preset | Java `DataOutputStream`: `i32 VERSION`, `i64 seed`, `writeUTF(configString)` (modified UTF-8 with a `u16` length), then the raw `customnames.rncn` bytes |
| `customnames.rncn` | UPR's custom names format, read and written byte-for-byte |
| Randomization log | Same text, same order, same number formatting |
| Check value | The `checkValue` that UPR computes and writes into the ROM must match |

Older settings versions are upgraded through a port of `SettingsUpdater.java`, so strings from any older UPR release keep loading. Strings from versions newer than 322 are refused with a clear message, the same way UPR handles them.

### 3.2 Seed parity, bit for bit

Same ROM + same settings + same seed must give a **byte-identical output ROM**, an identical log and the same check value. That requires reproducing Java behavior exactly:

- **`java.util.Random`:** the 48-bit LCG, the `setSeed` scrambling, `next(bits)`, the rejection loop in `nextInt(bound)`, `nextDouble`, `nextBoolean`, `nextLong`, `nextBytes`, and `nextGaussian` including its cached second value.
- **`RandomSource`:** two independent streams, main and cosmetic, both seeded from the same seed, with call order kept exactly. `pickSeed()` builds a 48-bit seed from 6 random bytes.
- **`Collections.shuffle(list, rnd)`:** the exact backwards swap loop.
- **Iteration order:** wherever UPR iterates a `HashMap` or `HashSet`, Randomon has to iterate in Java's bucket order. That means a `JavaHashMap` that copies Java's hashing, table capacity and resize behavior for `Integer`, `String` and enum keys. `LinkedHashMap` maps to `indexmap`, and `TreeMap` maps to `BTreeMap`.
- **Java semantics:** `Math.round`, integer overflow, integer division and remainder signs, `String.hashCode`, and how `Double.toString` and `String.format` print numbers in the log.
- **Upstream bugs:** where UPR has a bug that affects output, Randomon copies it in parity mode and marks it with a test.

### 3.3 Testing parity

- **`tools/vectors`:** a small Java harness that runs against the UPR 4.6.1 jar. It produces test vectors containing no game data: RNG sequences, shuffle results, settings-string round trips for every version, `.rnqs`/`.rndp` samples, and HashMap iteration orders. These are committed and run in CI on every platform.
- **`randomon-parity`:** a local or self-hosted harness. It takes your own ROMs and a grid of settings × seeds, runs UPR and Randomon on each, and diffs the output bytes, the log and the check value. UPR's CLI has no seed flag (only `-i -o -s -d -u -l`), so the harness uses a small Java wrapper that calls `Randomizer.randomize(file, log, seed)` directly. ROMs never go in the repo or on GitHub-hosted runners.
- **Per-generation exit criterion:** zero diffs across the full settings grid on every supported version of every game in that generation.

## 4. Architecture

Cargo workspace, edition 2024, laid out like Impactor:

```
randomon/
├── apps/
│   ├── randomon/            # iced GUI (default member)
│   └── randomon-cli/        # UPR-compatible CLI
├── crates/
│   ├── randomon_java/       # java.util.Random, shuffle, JavaHashMap, Java number formatting
│   ├── randomon_formats/    # containers, compression, text codecs, patching
│   ├── randomon_data/       # game models, per-gen constants, embedded offsets .ini and patches
│   ├── randomon_settings/   # Settings model, string codec, every version, rnqs/rndp/rncn
│   ├── randomon_core/       # engine: Randomizer + AbstractRomHandler port, RomHandler trait
│   ├── randomon_gen1 … gen7/# per-generation handlers
│   └── randomon_log/        # log writer with UPR-identical output
├── tools/
│   ├── vectors/             # Java test-vector generator (runs against the UPR jar)
│   └── parity/              # local differential harness
└── package/                 # NSIS, macOS bundle and DMG, AppImage and Flatpak manifests
```

**`randomon_formats`** covers:
- GB/GBC: header checksums.
- GBA: LZ77.
- NDS: NitroFS, NARC, ARM9 and overlays, LZ10/LZ11, BLZ, Y9 table.
- 3DS: NCSD, NCCH, ExeFS, RomFS/IVFC, CIA (for updates), GARC, Mini, AMX scripts, BFLIM, SMDH.
- IPS patching, `.tbl` text tables, Gen 4/5 PPTXT, and Gen 6/7 encrypted text.

Each format gets round-trip tests built from synthetic files.

**`randomon_data`** embeds UPR's `gen*_offsets.ini`, the `.tbl` files and `patches/` as-is, with GPL attribution. That keeps them easy to re-sync with upstream instead of turning them into Rust code by hand.

**`randomon_core`:** `RomHandler` becomes a trait, and `AbstractRomHandler`'s 7.5k lines of shared logic become default methods and helpers. The engine is pure, with no I/O. It works on in-memory `Rom` objects so the GUI, the CLI and the parity harness all share it.

**Size estimate:** about 77k lines of Java across 117 files. The biggest are `AbstractRomHandler` (7.6k) and the Gen 3–6 handlers (4.3–5.8k each). Expect roughly 50–60k lines of Rust.

## 5. GUI (iced 0.14)

Same stack as Impactor: `iced` (advanced features) + `wgpu`, `rfd` for file dialogs, `open` for links, `rust-i18n` + `current_locale`, and `notify-rust` for "randomization finished" notifications.

**Layout:**
- **Top bar:** a ROM card showing the game name, version, region, generation, and the icon or box art (from SMDH on 3DS). Drop a ROM on it or click "Open ROM". For 3DS games, a "Load update (.cia)" chip.
- **Left sidebar:** sections matching UPR's tabs, but cleaner:
  - Pokémon (base stats, types, abilities, evolutions)
  - Starters, Statics & Trades
  - Moves & Movesets
  - Foe Pokémon
  - Wild Pokémon
  - TMs/HMs & Tutors
  - Items
  - Misc Tweaks
- **Main pane:** grouped cards with toggles, segmented controls and sliders. Unsupported options are hidden for the loaded game rather than greyed out, with an "Unavailable for this game" disclosure. Each option has inline help taken from `Bundle.properties`.
- **Bottom action bar:**
  - Settings string field with copy and paste (pasting a string applies it immediately)
  - Seed field with a dice button
  - Load and save `.rnqs`
  - Load and make `.rndp` presets
  - A primary **Randomize** button with progress
- **After randomizing:** a log viewer with search and section jump, a "Save log" button, and a summary of what changed.
- **Extras:** batch randomization, a custom names editor, a race-mode indicator.
- **Theme:** light and dark that follow the OS, an accent color, and Inter or the system font.
- **Support:** a "Support Randomon ♥" button in the sidebar footer opens GitHub Sponsors and Ko-fi. The About page credits the UPR ZX team and Dabomstew and shows the GPL notice.

The CLI accepts UPR's `CliRandomizer` flags (`-i` input, `-o` output, `-s` settings file, `-d` save as directory, `-u` game update, `-l` save log), so existing scripts keep working. It adds `--seed` and `--settings-string`, which UPR's CLI doesn't have.

## 6. Repo setup

Matching your other repos:

- **`.github/FUNDING.yml`:**
  ```yaml
  github: castdrian
  ko_fi: castdrian
  ```
- **README header:** the same badge row as ishare (Sponsor, Ko-fi, Discord, GitHub), then build and release badges, screenshots, a download table, and a compatibility statement ("matches UPR ZX 4.6.1 settings and seeds").
- **`LICENSE`:** GPL-3.0-or-later. This is required because Randomon is derived from UPR ZX, which is GPLv3.
- **Other files:** `rust-toolchain.toml` (stable), `rustfmt.toml`, clippy lints in the workspace `Cargo.toml`, and `deny.toml` (license and advisory checks).
- **Dependabot** for `cargo` and `github-actions`, with an auto-merge workflow like dkrypt's.
- Issue templates (bug report asking for game, version, settings string and seed; feature idea) and a `CONTRIBUTING.md`.

## 7. GitHub Actions

Actions use major-version tags, like vbound. The repo's no-comments rule means a SHA pin couldn't carry its readable version tag. Releases ship automatically when the workspace version in `Cargo.toml` is bumped on `main`, using the same version check vbound does.

- **`ci.yml`** (push and PR):
  - `fmt --check` and `clippy -D warnings`
  - `cargo test` on Ubuntu, macOS and Windows
  - `cargo deny`
  - the test-vector suite
  - a GUI build check
- **`release.yml`** (called by CI after a version bump, or run manually):
  - Linux x86_64 and aarch64: AppImage + tarball (Flatpak optional)
  - Windows x86_64 and arm64: portable `.exe` + NSIS installer
  - macOS universal `.app` in a DMG, signed with Developer ID through `castdrian/setup-apple-certificate` and notarized, using the same secrets as vbound
  - CLI binaries for every target
  - SHA-256 checksums and a GitHub Release whose notes come from that version's CHANGELOG.md section
- **`parity.yml`** (optional): `workflow_dispatch` on a self-hosted runner that has your ROMs, running `tools/parity`.

## 8. Milestones

1. **M0: scaffold.** Workspace, CI, release pipeline producing empty app builds, README, FUNDING, license, and the GUI shell with sidebar, theme and Support button.
2. **M1: foundations.** `randomon_java` (RNG, shuffle, JavaHashMap, number formatting), the full settings codec with every historic version, `.rnqs`/`.rndp`/`.rncn`, and the Java test-vector generator. The GUI can already load and save settings and edit every option.
3. **M2: Gen 1 end to end.** The smallest container, used to prove the whole pipeline: port the shared engine (`Randomizer.java` and `AbstractRomHandler`) along with `Gen1RomHandler`, and get to zero parity diffs on RBY.
4. **M3: Gen 2, then Gen 3** (GBC, then GBA formats).
5. **M4: Gen 4 and Gen 5** (NDS: NitroFS, NARC, overlays, PPTXT).
6. **M5: Gen 6 and Gen 7** (3DS: NCCH, RomFS, GARC, AMX, CIA updates, LayeredFS output).
7. **1.0:** every generation at parity, signed releases, translations started.

**After 1.0:** optional Randomon-only features, kept out of parity mode. One example: the encounter-file repacking issue from today, where Gen 6 output could pack data back to its original size.

## 9. Risks

- **Getting exact parity:** iteration order, log number formatting, and copying upstream bugs. This is handled by porting closely first and refactoring later, with test vectors, and with a parity diff at every milestone.
- **Upstream changes:** compatibility is pinned to 4.6.1. Each new UPR release becomes a tracked issue to re-sync the offsets, patches and settings version.
- **Legal:** the license is GPLv3, with no ROMs, keys or BIOS files in the repo or in CI. Users provide their own games.
- **3DS output on real hardware:** UPR already works around some instructions that only run correctly in emulators. Those workarounds have to be ported exactly.

## 10. Brilliant Diamond and Shining Pearl

BDSP support is derived from [Ai0796/BDSP-Randomizers](https://github.com/Ai0796/BDSP-Randomizers) (MIT), not UPR ZX, because UPR doesn't support Switch games.

**What the source does.** About 5k lines of Python. It edits Unity asset bundles with UnityPy: `masterdatas`, `personal_masterdatas`, `ev_script`, `gamesettings`, `UgData` and the English message bundle. It writes per-version ARM64 exefs patches for 1.0.0, 1.1.0–1.1.3, 1.2.0 and 1.3.0, and outputs Atmosphere, Yuzu or Ryujinx mod folders.

**Features to port:**
- starters, wild encounters (generation pools, legendaries, Safari) and Underground encounters
- encounter level scaling
- trainers, movesets, TMs and TM compatibility
- abilities, evolutions, field items and shops
- the warp randomizer
- the extra mods: HGSS walk speed, true size and weather removal

**Compatibility contract.** The source uses Python's unseeded `random`, so there are no seeds or settings files to stay compatible with. Instead:
- Randomon exactly reproduces Python's MT19937 `random`: `seed`, `random`, `randrange`/`randint` via `getrandbits`, `choice`, `shuffle` and `sample`.
- A reference harness runs the original code with `random.seed(n)` injected. For the same seed, Randomon must produce identical decoded asset contents: every edited MonoBehaviour field, the script commands and the patch bytes.
- Bundle container bytes, such as re-compression, are compared only after decoding. Seeds become reproducible for BDSP, which the original never offered.

**Architecture:**
- `randomon_unity`: a UnityFS bundle reader and writer (LZ4/LZMA blocks, SerializedFile, typetree-driven object reading and writing) that round-trips unmodified bundles losslessly.
- `randomon_bdsp`: the feature ports, the version detection that picks the exefs patch set, and the mod-folder output layouts.
- `randomon_settings::bdsp`: a Randomon-native settings model with its own versioned string format. It stays separate from the UPR string so UPR parity is unaffected.

**Input.** An extracted romfs folder, as the original expects. NSP/XCI extraction needs the user's keys and is out of scope.

**Future.** Implement `RomHandler` for BDSP so UPR ZX-style settings can drive BDSP too.

## 11. Decisions

1. **License:** GPL-3.0-or-later.
2. **Parity:** byte-identical output ROM, log and check value.
3. **macOS signing:** Developer ID and notarization, using vbound's secrets (`APP_STORE_CONNECT_KEY_ID`, `APP_STORE_CONNECT_ISSUER_ID`, `APP_STORE_CONNECT_PRIVATE_KEY_BASE64`, `DISTRIBUTION_PRIVATE_KEY_BASE64`). Bundle ID `dev.adrian.randomon`.
4. **Discord:** `https://discord.gg/NdaBaxFKnn`, the same invite as DeArrow.
