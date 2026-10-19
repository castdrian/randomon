## [1.3.1](https://github.com/castdrian/randomon/compare/v1.3.0...v1.3.1) (2026-10-19)

### Bug Fixes

* write Switch mods for the loaded game only, without a mod name folder, and hide options a game doesn't have ([141fc1f](https://github.com/castdrian/randomon/commit/141fc1f13a2ead36c37855bf5f7c6a97d31a00eb))

## [1.3.0](https://github.com/castdrian/randomon/compare/v1.2.2...v1.3.0) (2026-10-19)

### Features

* randomize Brilliant Diamond, Shining Pearl, Sword and Shield with the full option catalog ([5e26b58](https://github.com/castdrian/randomon/commit/5e26b58da157262a330eb4bfc864efd889f5cfb3))

## [1.2.2](https://github.com/castdrian/randomon/compare/v1.2.1...v1.2.2) (2026-10-19)

### Bug Fixes

* **app:** load large DS ROMs such as Black and White instead of showing a Generation 4 preview ([220b30b](https://github.com/castdrian/randomon/commit/220b30bdf48c7f2e7378301d49fde87f493b5745))
* **deps:** update rustls to 0.23.46 for its security fixes ([c02b11e](https://github.com/castdrian/randomon/commit/c02b11e5d66621bbb9a13e077aef497612c7d339))

## [1.2.1](https://github.com/castdrian/randomon/compare/v1.2.0...v1.2.1) (2026-10-12)

### Bug Fixes

* **cli:** only warn about unavailable starters when custom starters were chosen ([1890bbb](https://github.com/castdrian/randomon/commit/1890bbb4bb152214e2d3ac10c39efe8ba3dc276b))

## [1.2.0](https://github.com/castdrian/randomon/compare/v1.1.1...v1.2.0) (2026-10-12)

### Features

* **app:** add a built-in self updater backed by GitHub releases ([25abec6](https://github.com/castdrian/randomon/commit/25abec6a572fe65a84a07ed4c35efa1d5f073c02))
* **app:** stream randomization progress and show a clear result card ([eadd3f1](https://github.com/castdrian/randomon/commit/eadd3f1a46139c3bfcc851a5a754bbed7fe7ea4c))

### Bug Fixes

* **app:** show the first progress detail of every stage, report LayeredFS writes, tidy the loaded status and refresh the screenshot ([ca68277](https://github.com/castdrian/randomon/commit/ca68277754694d075bf72e7e4da15731c19ddad1))
* **bdsp:** name the current Switch emulators, Eden and Ryubing, in layouts and the README ([9764460](https://github.com/castdrian/randomon/commit/97644606d96d5ee4d30cf0a8402a2444361f3105))
* **build:** vendor block with the uninhabited static fixed so builds have no future-incompat warnings ([06af156](https://github.com/castdrian/randomon/commit/06af1560e010d290bf9a14aa41f7240189830d60))

## [1.1.1](https://github.com/castdrian/randomon/compare/v1.1.0...v1.1.1) (2026-10-11)

### Bug Fixes

* **app:** keep the Open game button visible and use neutral open wording ([8a42b5f](https://github.com/castdrian/randomon/commit/8a42b5fbc6b7e65f26a694598a9b358c71b4f8ed))

## [1.1.0](https://github.com/castdrian/randomon/compare/v1.0.0...v1.1.0) (2026-10-11)

### Features

* **3ds:** decrypt encrypted NCCH ROMs and CIA updates transparently ([5376edb](https://github.com/castdrian/randomon/commit/5376edb536db79ac4b39f6cdd9f9f00fe8cde602))
* **3ds:** search iCloud Drive for keys and let the app choose a keys file ([1d2e6f0](https://github.com/castdrian/randomon/commit/1d2e6f0256ad48f6144c8622148c0c65410e5903))
* **app:** unify game opening, add update chip and settings sharing dialog ([4ffb605](https://github.com/castdrian/randomon/commit/4ffb6052118f14d4fdbc24320cb3aa4956e0a7b3))

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
