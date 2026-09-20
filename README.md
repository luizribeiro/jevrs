# jevrs

[![crates.io](https://img.shields.io/crates/v/jevrs.svg)](https://crates.io/crates/jevrs)
[![docs.rs](https://docs.rs/jevrs/badge.svg)](https://docs.rs/jevrs)
[![CI](https://github.com/luizribeiro/jevrs/actions/workflows/ci.yml/badge.svg)](https://github.com/luizribeiro/jevrs/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/crates/msrv/jevrs.svg)](https://crates.io/crates/jevrs)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

`jevrs` is an async Rust client for `TypeSafe` AI's Jev (System One) API. It gives you typed question sets and answers, a runtime builder, configurable retries, and native or WASI transports over a portable sans-I/O core.

Set `TYPESAFE_API_KEY`, then:

```rust,no_run
use jevrs::{Choice, Client, Levels, Noul, Options, Questions, Score};

#[derive(Clone, Copy, Debug, Eq, Options, PartialEq)]
enum Department {
    /// Payments, invoicing, refunds
    Billing,
    /// Bugs, outages, integrations
    Technical,
    Sales,
}

#[derive(Clone, Copy, Debug, Eq, Levels, Ord, PartialEq, PartialOrd)]
enum Frustration {
    /// Calm
    Calm,
    /// Frustrated
    Frustrated,
    /// Very angry
    VeryAngry,
}

#[derive(Questions)]
struct Triage {
    #[jev(
        noul = "Does this convey urgency?",
        yes = "Explicitly time-sensitive",
        no = "No urgency expressed"
    )]
    is_urgent: Noul,
    #[jev(choice = "Which team should handle this?")]
    department: Choice<Department>,
    #[jev(score = "How frustrated is the customer?")]
    frustration: Score<Frustration>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    let triage = client
        .ask::<Triage>(&"Help! My payouts have been failing for 3 days.")
        .await?;

    println!("urgent: {:.0}%", triage.is_urgent.p.get() * 100.0);
    println!("department: {:?}", triage.department.pick);
    println!("frustration: {:.2}", triage.frustration.expected());
    Ok(())
}
```

Install with `cargo add jevrs tokio --features tokio/macros,tokio/rt-multi-thread`.

Read the [API documentation](https://docs.rs/jevrs), then choose the guide for the [typed path](https://docs.rs/jevrs/latest/jevrs/#the-typed-path), [dynamic path](https://docs.rs/jevrs/latest/jevrs/#the-dynamic-path), [transports](https://docs.rs/jevrs/latest/jevrs/#transports), or [retries and errors](https://docs.rs/jevrs/latest/jevrs/#retries-and-errors).

Examples: [typed native](examples/native-derive), [dynamic native](examples/native-dynamic), [WASI HTTP 0.2](examples/wasip2), and [WASI HTTP 0.3](examples/wasip3).

## Comparison with other Jev crates

Several community crates wrap the same API; none is official. This table
reflects each crate's documentation as of 2026-09-20.

| Crate | Typed answers | Custom transport | WASI | Sync client |
|---|---|---|---|---|
| `jevrs` | `Options`, `Levels`, and `Questions` derives, plus a runtime builder | `Transport` trait over a sans-I/O core | wasip2, wasip3 | no |
| [`typesafe-sdk-client`](https://crates.io/crates/typesafe-sdk-client) | `derive(Questions)` | no, reqwest | no | no |
| [`typesafe-client`](https://crates.io/crates/typesafe-client) | choice questions only, via `choice_options!` | no, reqwest | no | no |
| [`typesafe-rs`](https://crates.io/crates/typesafe-rs) | no, string builder | no, reqwest | no | reqwest blocking |
| [`typesafe-ai`](https://crates.io/crates/typesafe-ai) | no, JSON maps | reqwest or ureq backends | no | yes, ureq |
| [`kunobi-jev`](https://crates.io/crates/kunobi-jev) | no, string arrays | no, reqwest | no | no |

Licensed under either [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option.
