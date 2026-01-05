# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.5](https://github.com/szaffarano/org-mcp-server/compare/org-core-v0.0.4...org-core-v0.0.5)

### 🐛 Bug Fixes


- Update integration test ([#117](https://github.com/szaffarano/org-mcp-server/pull/117)) - ([b282630](https://github.com/szaffarano/org-mcp-server/commit/b2826308cff22c5ac424f2a6ea769f885d18718e))
- Update integration test flakiness ([#108](https://github.com/szaffarano/org-mcp-server/pull/108)) - ([3c2aee1](https://github.com/szaffarano/org-mcp-server/commit/3c2aee153453d9e47bc29d13d217fa6e7d50e88e))

### 🚜 Refactor


- Move integration tests to tests/ directory ([#100](https://github.com/szaffarano/org-mcp-server/pull/100)) - ([4cb31b1](https://github.com/szaffarano/org-mcp-server/commit/4cb31b180b6f6b87155a6a9e49c849fb57e1525b))

### ⚙️ Miscellaneous Tasks


- Consolidate test fixtures into test-utils crate ([#99](https://github.com/szaffarano/org-mcp-server/pull/99)) - ([6101d25](https://github.com/szaffarano/org-mcp-server/commit/6101d25baa9429deb8b7343767cdb7a5016fbaec))

### Other changes


- Update configuration sample and path in README ([#120](https://github.com/szaffarano/org-mcp-server/pull/120)) - ([fa9ad89](https://github.com/szaffarano/org-mcp-server/commit/fa9ad89043f4ff2e38703f84daaef72d8c664b35))


## [0.0.4](https://github.com/szaffarano/org-mcp-server/compare/org-core-v0.0.3...org-core-v0.0.4)

### ✨ Features


- Implement agenda view functionality ([#86](https://github.com/szaffarano/org-mcp-server/pull/86)) - ([87e412d](https://github.com/szaffarano/org-mcp-server/commit/87e412dbcf5136972afb74ee0dd97400e3f883a3))

### Other changes


- Add cargo install and GitHub releases to setup instructions ([#81](https://github.com/szaffarano/org-mcp-server/pull/81)) - ([2b543ed](https://github.com/szaffarano/org-mcp-server/commit/2b543edfe29cc1d652f9b1880a09eecb4a41f432))


## [0.0.3](https://github.com/szaffarano/org-mcp-server/compare/org-core-v0.0.2...org-core-v0.0.3)

### ✨ Features


- *(mcp)* Add tags parameter to list files tool ([#79](https://github.com/szaffarano/org-mcp-server/pull/79)) - ([1b02b24](https://github.com/szaffarano/org-mcp-server/commit/1b02b243cd9d3f478cd46b682183e098b408c93f))

### 🚜 Refactor


- *(config)* [**breaking**] Improve configuration architecture ([#78](https://github.com/szaffarano/org-mcp-server/pull/78)) - ([4053d2e](https://github.com/szaffarano/org-mcp-server/commit/4053d2e8655125ec712438d021c89c768dd0962b))


## [0.0.2](https://github.com/szaffarano/org-mcp-server/compare/org-core-v0.0.1...org-core-v0.0.2) - 2025-10-12

### Added

- add tag filtering support for search and list operations ([#71](https://github.com/szaffarano/org-mcp-server/pull/71))

### Other

- convert README from org-mode to markdown ([#72](https://github.com/szaffarano/org-mcp-server/pull/72))
- prepare crate metadata for crates.io publishing ([#67](https://github.com/szaffarano/org-mcp-server/pull/67))

## [0.0.1](https://github.com/szaffarano/org-mcp-server/releases/tag/org-core-v0.0.1) - 2025-10-06

### Added

- add TOML-based configuration system ([#45](https://github.com/szaffarano/org-mcp-server/pull/45))
- Implement search tool ([#30](https://github.com/szaffarano/org-mcp-server/pull/30))
- *(mcp-server)* Rename binary ([#24](https://github.com/szaffarano/org-mcp-server/pull/24))
- *(mcp-server)* Update resource uri ([#23](https://github.com/szaffarano/org-mcp-server/pull/23))
- *(test)* Improve code coverage ([#20](https://github.com/szaffarano/org-mcp-server/pull/20))
- Add coverage ([#11](https://github.com/szaffarano/org-mcp-server/pull/11))
- *(mcp-server)* Add root flag ([#7](https://github.com/szaffarano/org-mcp-server/pull/7))

### Fixed

- *(deps)* update rust crate rmcp to 0.8.0 ([#42](https://github.com/szaffarano/org-mcp-server/pull/42))
- *(deps)* update rust crate rowan to 0.16 ([#18](https://github.com/szaffarano/org-mcp-server/pull/18))

### Other

- Prepare initial release ([#65](https://github.com/szaffarano/org-mcp-server/pull/65))
- release v0.0.1 ([#63](https://github.com/szaffarano/org-mcp-server/pull/63))
- rename mcp-server crate to org-mcp-server ([#64](https://github.com/szaffarano/org-mcp-server/pull/64))
- prepare for initial 0.0.1 release ([#62](https://github.com/szaffarano/org-mcp-server/pull/62))
- release v0.2.0 ([#60](https://github.com/szaffarano/org-mcp-server/pull/60))
- *(core)* fix flaky config tests with proper env isolation ([#49](https://github.com/szaffarano/org-mcp-server/pull/49))
- *(deps)* update rust crate toml to 0.9 ([#46](https://github.com/szaffarano/org-mcp-server/pull/46))
- *(docs)* Update readme ([#33](https://github.com/szaffarano/org-mcp-server/pull/33))
- Initial commit
