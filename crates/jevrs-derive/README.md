# jevrs-derive

[![crates.io](https://img.shields.io/crates/v/jevrs-derive.svg)](https://crates.io/crates/jevrs-derive)
[![docs.rs](https://docs.rs/jevrs-derive/badge.svg)](https://docs.rs/jevrs-derive)

Derive macros for [`jevrs`](https://crates.io/crates/jevrs): `Options` for a
set of named choices, `Levels` for an ordered scoring scale, and `Questions`
for a typed question set.

Do not depend on this crate directly. `jevrs` re-exports the three derives
behind its default `derive` feature. Generated code refers to `jevrs` items by
default; the `#[jev(crate = "path")]` container attribute overrides that path.
See the [`jevrs` README](https://crates.io/crates/jevrs) for a complete example.

Licensed under either [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option.
