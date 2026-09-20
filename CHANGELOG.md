# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- Milestone 1 native client: core question and answer types, dynamic builders,
  JSON codec, async client with retries, reqwest transport, recorded live API
  fixtures, and the `native-dynamic` example.
- `Options` and `Levels` derives for static choice and scoring criteria, enabled
  by the default `derive` feature.
- Typed question sets with `derive(Questions)`, `Answered<T>`, `Client::ask`,
  and the `native-derive` example.
