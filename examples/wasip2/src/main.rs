//! Shows the typed triage flow with derives in a WASI HTTP 0.2 component.
//!
//! Build and run against the live API:
//!
//! ```text
//! cargo build -p wasip2-example --target wasm32-wasip2 && wasmtime run -S http --env TYPESAFE_API_KEY target/wasm32-wasip2/debug/wasip2_example.wasm
//! ```
//!
//! Run against the mock with `cargo xtask mock --port 3000` in one terminal,
//! then:
//!
//! ```text
//! cargo build -p wasip2-example --target wasm32-wasip2 && wasmtime run -S http --env TYPESAFE_API_KEY=test-key --env TYPESAFE_BASE_URL=http://127.0.0.1:3000 target/wasm32-wasip2/debug/wasip2_example.wasm
//! ```

#[cfg(target_arch = "wasm32")]
use jevrs::{Choice, Client, Levels, Noul, Options, Questions, Score};

#[cfg(target_arch = "wasm32")]
const STATE: &str = "Help! My payouts have been failing for 3 days.";

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug, Eq, Options, PartialEq)]
enum Department {
    /// Payments, invoicing, refunds
    Billing,
    /// Bugs, outages, integrations
    Technical,
    Sales,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug, Eq, Levels, Ord, PartialEq, PartialOrd)]
enum Frustration {
    /// Calm
    Calm,
    /// Frustrated
    Frustrated,
    /// Very angry
    VeryAngry,
}

#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "wasm32")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    futures::executor::block_on(run())
}

#[cfg(target_arch = "wasm32")]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::wasip2().from_env()?.build()?;
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

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("build with: cargo build -p wasip2-example --target wasm32-wasip2");
    std::process::exit(1);
}
