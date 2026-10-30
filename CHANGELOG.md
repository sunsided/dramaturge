# Changelog

All notable changes to this project are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased] — 0.2.0

### Added

- `SymbolTable` and `Interner` in `dramaturge-model`: stable author keys mapped
  to dense runtime ids, one namespace per id kind (#55).
- `GraphValidator::validate_with_symbols` names nodes and options by key (#55).
- CI lints GitHub Actions workflows with actionlint and zizmor.

### Changed

- **Breaking:** the YAML format is now 2.0. All references are keys
  (`target: mara.confession.soft`, `source: player`). Numeric ids still load
  for one release with a deprecation warning (#55).
- **Breaking:** `YamlDialogueGraph::to_model` returns `LoadedDialogue
  { graph, symbols, warnings }` (#55).
- Validator messages print ids as `'key'` or `#<number>` instead of `ID <number>`.
