# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.1.0] - 2026-09-20

### Added

- Milestone 1 native client: core question and answer types, dynamic builders,
  JSON codec, async client with retries, reqwest transport, recorded live API
  fixtures, and the `native-dynamic` example.
- `Options` and `Levels` derives for static choice and scoring criteria, enabled
  by the default `derive` feature.
- Typed question sets with `derive(Questions)`, `Answered<T>`, `Client::ask`,
  and the `native-derive` example.
- Milestone 3 WASI HTTP 0.2 transport, monotonic-clock retry sleep,
  `Client::wasip2()`, runnable component example, and fixture-backed Wasmtime
  smoke test.
- Milestone 4 WASI HTTP 0.3 async transport, monotonic-clock retry sleep,
  `Client::wasip3()`, runnable component example, and fixture-backed Wasmtime
  smoke test on the pinned nightly toolchain.
- String, object, and array question instructions with verbatim JSON encoding.
- Dependency license, source, wildcard, duplicate-version, and advisory checks.
- Crate metadata, docs.rs configuration, license files, and package verification
  for the 0.1.0 release.
