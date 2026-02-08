# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
