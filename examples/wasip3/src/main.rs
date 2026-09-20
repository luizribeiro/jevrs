//! Shows the typed triage flow with derives in a WASI HTTP 0.3 component.
//!
//! This example needs a nightly toolchain with the `wasm32-wasip3` target;
//! this repository's `nix develop .#nightly` provides one.
//!
//! Build and run against the live API:
//!
//! ```text
//! nix develop .#nightly -c cargo build -p wasip3-example --target wasm32-wasip3 && wasmtime run -S http --env TYPESAFE_API_KEY target/wasm32-wasip3/debug/wasip3_example.wasm
//! ```
//!
//! Run against the mock with `cargo xtask mock --port 3000` in one terminal,
//! then:
//!
//! ```text
//! nix develop .#nightly -c cargo build -p wasip3-example --target wasm32-wasip3 && wasmtime run -S http --env TYPESAFE_API_KEY=test-key --env TYPESAFE_BASE_URL=http://127.0.0.1:3000 target/wasm32-wasip3/debug/wasip3_example.wasm
//! ```

#[cfg(all(target_arch = "wasm32", target_env = "p3"))]
use jevrs::{Choice, Client, Levels, Noul, Options, Questions, Score};

#[cfg(all(target_arch = "wasm32", target_env = "p3"))]
const STATE: &str = "Help! My payouts have been failing for 3 days.";

#[cfg(all(target_arch = "wasm32", target_env = "p3"))]
#[derive(Clone, Copy, Debug, Eq, Options, PartialEq)]
enum Department {
    /// Payments, invoicing, refunds
    Billing,
    /// Bugs, outages, integrations
    Technical,
    Sales,
}

#[cfg(all(target_arch = "wasm32", target_env = "p3"))]
#[derive(Clone, Copy, Debug, Eq, Levels, Ord, PartialEq, PartialOrd)]
enum Frustration {
    /// Calm
    Calm,
    /// Frustrated
    Frustrated,
    /// Very angry
    VeryAngry,
}

#[cfg(all(target_arch = "wasm32", target_env = "p3"))]
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

#[cfg(all(target_arch = "wasm32", target_env = "p3"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    wasip3::wit_bindgen::block_on(run())
}

#[cfg(all(target_arch = "wasm32", target_env = "p3"))]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::wasip3().from_env()?.build()?;
    let answers = client.ask::<Triage>(&STATE).await?;

    println!("model: {}", answers.model());
    println!("urgent probability: {:.2}", answers.is_urgent.p.get());

    let department = &answers.department;
    println!(
        "department: {} (confidence {:.2})",
        department.pick.key(),
        department.confidence.get()
    );
    for (option, probability) in department.probs.iter() {
        println!("  {}: {:.2}", option.key(), probability.get());
    }

    let frustration = &answers.frustration;
    println!(
        "frustration: {:.2} ({})",
        frustration.expected(),
        frustration.nearest().description()
    );

    let usage = answers.usage();
    println!(
        "usage: {} input tokens, {} output tokens",
        usage.input_tokens, usage.output_tokens
    );
    Ok(())
}

#[cfg(not(all(target_arch = "wasm32", target_env = "p3")))]
fn main() {
    eprintln!(
        "build with: nix develop .#nightly -c cargo build -p wasip3-example --target wasm32-wasip3"
    );
    std::process::exit(1);
}
