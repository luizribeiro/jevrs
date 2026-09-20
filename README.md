# jevrs

`jevrs` is an async Rust client for `TypeSafe` AI's Jev (System One) API. It gives you typed question sets and answers, a runtime builder, configurable retries, and native or WASI transports over a portable sans-I/O core.

`typesafe-rs` is the other Rust client. It requires reqwest and Tokio and builds questions from strings. `jevrs` has a sans-I/O core, typed questions and answers through derives, and WASI transports.

Set `TYPESAFE_API_KEY`, then:

```no_run
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
    let client = Client::reqwest().from_env()?.build()?;
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

Licensed under either [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option.
