# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0](https://github.com/Bli-AIk/bevy_alight_motion/compare/v0.4.0...v0.5.0) - 2026-06-30

### Refactor

- *(core)* extract AM data layer into am_core dylib crate + FFI module split ([#47](https://github.com/Bli-AIk/bevy_alight_motion/pull/47))

## [0.4.0](https://github.com/Bli-AIk/bevy_alight_motion/compare/v0.3.1...v0.4.0) - 2026-04-19

### Added

- RTT performance optimization, rendering fixes, and test infrastructure improvements ([#36](https://github.com/Bli-AIk/bevy_alight_motion/pull/36))
- *(renderer)* add effect coverage and split oversized runtime modules ([#34](https://github.com/Bli-AIk/bevy_alight_motion/pull/34))
- *(effects)* add z_qq_group_logo basic project
- *(loader)* support unpacked .amproj directories and content URIs with override config ([#30](https://github.com/Bli-AIk/bevy_alight_motion/pull/30))
- *(loader)* support unpacked .amproj directories and content URIs
- group fill, jitter/echo keyframes, embed retime, and SDF fixes ([#28](https://github.com/Bli-AIk/bevy_alight_motion/pull/28))

### Fixed

- update example files

### Miscellaneous Tasks

- *(lint)* improve #[expect] reason detection in tokei scripts

### Refactor

- *(loader)* use is_some_and for extension check
- *(scene)* split spawn module into specialized files

## [0.3.1](https://github.com/Bli-AIk/bevy_alight_motion/compare/v0.3.0...v0.3.1) - 2026-02-26

### Added

- add multiple new effects and comprehensive repeat support ([#26](https://github.com/Bli-AIk/bevy_alight_motion/pull/26))
- implement pixelate effect and fix video resolution mismatch

### Miscellaneous Tasks

- *(test, docs)* update test results and documentation timestamps

### Refactor

- *(animation)* fix linear repeat offset and add debug logs

## [0.3.0](https://github.com/Bli-AIk/bevy_alight_motion/compare/v0.2.0...v0.3.0) - 2026-02-11

### Added

- [**breaking**] upgrade to bevy 0.18
- implement repeat, linear-repeat and other visual effects ([#19](https://github.com/Bli-AIk/bevy_alight_motion/pull/19))

### Documentation

- *(readme)* update Bevy version support and dependencies
- *(bevy_alight_motion)* add language auto-redirect to docs index

### Miscellaneous Tasks

- *(bevy_alight_motion)* update .gitignore path for assets

### Refactor

- *(player)* update imports and fix feature flag logic
- *(examples)* update player.rs

## [0.2.0](https://github.com/Bli-AIk/bevy_alight_motion/compare/v0.1.1...v0.2.0) - 2026-02-08

### Added

- add validation system, WASM playground, and automated documentation generation ([#17](https://github.com/Bli-AIk/bevy_alight_motion/pull/17))

### Documentation

- restructure vitepress config and update landing pages
- update playground index links to reference effects
- Initialize comprehensive bilingual documentation with custom VitePress theme ([#14](https://github.com/Bli-AIk/bevy_alight_motion/pull/14))

### Fixed

- *(docs)* use withBase for WASM asset paths in playground

### Miscellaneous Tasks

- *(bevy_alight_motion)* update gitignore for complex assets

### Refactor

- *(examples)* format multi-line function calls for readability
- reorganize example files into hierarchical folder structure

## [0.1.1](https://github.com/Bli-AIk/bevy_alight_motion/compare/v0.1.0...v0.1.1) - 2026-02-06

### Miscellaneous Tasks

- *(ci)* update GitHub Actions to latest versions

### Refactor

- *(bevy_alight_motion)* replace bevy_smud with custom SdfMaterial
- *(shaders)* replace smoothstep with step for pixel-perfect rendering
