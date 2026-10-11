## 1.0.0 (2026-10-11)

### Features

* **bdsp:** randomize Brilliant Diamond and Shining Pearl from a romfs folder ([733f671](https://github.com/castdrian/randomon/commit/733f67132110c3df57f06e1383eaf444ddb63f56))
* **gen3:** port the Ruby, Sapphire, Emerald, FireRed and LeafGreen handler ([569267f](https://github.com/castdrian/randomon/commit/569267f41a97fcfc89a2e8cc7b8df0dd1ca7ae15))
* **gen3:** register the Gen 3 handler and add synthetic, real-ROM and parity tests ([8a9f6c7](https://github.com/castdrian/randomon/commit/8a9f6c7ed7d221817b6c96379355d9fd02abc2d1))
* **gen4:** port Gen4RomHandler, Gen 4 constants, offsets and patches ([46b91fd](https://github.com/castdrian/randomon/commit/46b91fd1e4df6f91365043b1521e4ae4355735a2))
* support trainer held items and movesets on Gen 2 and extra trainer Pokemon on Gen 1 and 2 ([d7819ff](https://github.com/castdrian/randomon/commit/d7819ffe89d2b72bf795bd1826dd43b5e9cf3026))

### Bug Fixes

* **app:** catch engine panics and report them instead of crashing ([3f7b617](https://github.com/castdrian/randomon/commit/3f7b617cfeb345fea7112ab37e61ccf3c324992a))
* **gen1:** pack trainers, movesets and evolutions into shared bank space ([f91a612](https://github.com/castdrian/randomon/commit/f91a6126ec63cf6f2ee98f4cee1d5df89fc4c0c6))
* **gen2:** repack trainer data into the bank's free space and trim growth safely ([9c2d633](https://github.com/castdrian/randomon/commit/9c2d633fe079c9fa5544d412ca54f8351926516d))
* **gen3:** harden against corrupted ROMs and document Gen 3 departures from upstream ([d66a286](https://github.com/castdrian/randomon/commit/d66a28663685b043db56a06be8ea5222a7fde5b9))
* **gen4:** keep HGSS headbutt and contest slots in place, harden synthetic validation and document Gen 4 deviations ([db08e8d](https://github.com/castdrian/randomon/commit/db08e8daead11fa1bef868693f3d4d7c906c9a3d))
* handle typeless moves in trainer movesets, alt-forme evolution targets and cycle-safe chains ([a7ff69b](https://github.com/castdrian/randomon/commit/a7ff69b0413bfc67b051d98f7c004fb965f88c4a))
* surface handler failures as errors instead of panics and widen similar-strength searches safely ([40d15d6](https://github.com/castdrian/randomon/commit/40d15d600631225c14696d57dc914f04329d026f))

# Changelog

All notable changes to Randomon are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Workspace, app shell, CLI shell, CI and release pipeline.
