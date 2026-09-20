# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Changed

- `jevrs-core` and `jevrs-derive` ship their own READMEs on crates.io.
- The `jevrs` guide explains every way to construct a client, not only
  `Client::from_env`.

## [0.1.0] - 2026-09-20

### Added

- Native client: core question and answer types, dynamic builders,
  JSON codec, async client with retries, `Client::from_env`, `Client::builder`,
  target-selected `DefaultTransport`, and the `native-dynamic` example.
- Dynamic builder methods that accept raw criteria items and report the
  question ID in `Error::InvalidCriteria`.
- `Options` and `Levels` derives for static choice and scoring criteria, enabled
  by the default `derive` feature.
- Typed question sets with `derive(Questions)`, `Answered<T>`, `Client::ask`,
  and the `native-derive` example.
- WASI HTTP 0.2 transport selected on `wasm32-wasip2`, monotonic-clock retry
  sleep, and a runnable component example.
- WASI HTTP 0.3 async transport selected on `wasm32-wasip3`, monotonic-clock
  retry sleep, and a runnable component example.
- String, object, and array question instructions with verbatim JSON encoding.
- Crate metadata, docs.rs configuration, license files, and package verification
  for the 0.1.0 release.
- Documentation: crate guides for typed and dynamic workflows, public API
  examples, transport and retry guidance, and runnable example links.
