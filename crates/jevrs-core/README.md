# jevrs-core

[![crates.io](https://img.shields.io/crates/v/jevrs-core.svg)](https://crates.io/crates/jevrs-core)
[![docs.rs](https://docs.rs/jevrs-core/badge.svg)](https://docs.rs/jevrs-core)

Portable sans-I/O types, builders, and codecs for TypeSafe AI's Jev (System
One) API. The crate is `#![no_std]` with `alloc` and depends only on `serde`,
`serde_json`, and `thiserror`.

Use it directly when you already have an HTTP stack: build a `Questions`
batch, encode the request body, send it however you like, and decode the
response. Most applications should use [`jevrs`](https://crates.io/crates/jevrs)
instead; it adds the derives, an async client with retries, and native and
WASI transports on top of this crate.

```rust
use jevrs_core::{Model, Questions, decode, encode};

fn main() -> Result<(), jevrs_core::Error> {
    let mut questions = Questions::new();
    let urgent = questions.noul("is_urgent", "Does this convey urgency?")?;
    let request = encode(
        &Model::LATEST,
        &"Help! My payouts have been failing for 3 days.",
        &questions,
    )?;
    assert!(!request.is_empty());

    let response = br#"{
      "model":"jev-1.13.0",
      "answers":{"is_urgent":{"type":"noul","noul":0.95}},
      "usage":{"input_tokens":414,"output_tokens":73}
    }"#;
    let answers = decode(&questions, response)?;
    assert!(answers.get(urgent).is_yes(0.9));
    Ok(())
}
```

Licensed under either [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option.
