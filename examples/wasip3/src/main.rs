//! Demonstrates runtime-defined triage questions in a WASI HTTP 0.3 component.
//!
//! Run live with:
//! `nix develop .#nightly -c cargo build -p wasip3-example --target wasm32-wasip3 && wasmtime run -S http --env TYPESAFE_API_KEY target/wasm32-wasip3/debug/wasip3_example.wasm`
//!
//! Run against the mock with `cargo xtask mock --port 3000` in one terminal,
//! then:
//! `nix develop .#nightly -c cargo build -p wasip3-example --target wasm32-wasip3 && wasmtime run -S http --env TYPESAFE_API_KEY=test-key --env TYPESAFE_BASE_URL=http://127.0.0.1:3000 target/wasm32-wasip3/debug/wasip3_example.wasm`

#[cfg(all(target_arch = "wasm32", target_env = "p3"))]
use jevrs::{Client, DynLevels, DynOptions, Questions};

#[cfg(all(target_arch = "wasm32", target_env = "p3"))]
const STATE: &str = "Help! My payouts have been failing for 3 days.";

#[cfg(all(target_arch = "wasm32", target_env = "p3"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    wasip3::wit_bindgen::block_on(run())
}

#[cfg(all(target_arch = "wasm32", target_env = "p3"))]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut questions = Questions::new();
    let urgent = questions.noul_with(
        "is_urgent",
        "Does this convey urgency?",
        "Explicitly time-sensitive",
        "No urgency expressed",
    )?;
    let department = questions.choice_dyn(
        "department",
        "Which team should handle this?",
        DynOptions::new([
            (
                "billing".into(),
                Some("Payments, invoicing, refunds".into()),
            ),
            (
                "technical".into(),
                Some("Bugs, outages, integrations".into()),
            ),
            ("sales".into(), None),
        ])?,
    )?;
    let frustration = questions.score_dyn(
        "frustration",
        "How frustrated is the customer?",
        DynLevels::new(["Calm".into(), "Frustrated".into(), "Very angry".into()])?,
    )?;

    let client = Client::wasip3().from_env()?.build()?;
    let answers = client.evaluate(&STATE, &questions).await?;

    println!("model: {}", answers.model());
    println!("urgent probability: {:.2}", answers[urgent].p.get());

    let department = &answers[department];
    println!(
        "department: {} (confidence {:.2})",
        department.pick,
        department.confidence.get()
    );
    for (option, probability) in &department.probs {
        println!("  {option}: {:.2}", probability.get());
    }

    let frustration = &answers[frustration];
    println!(
        "frustration: {:.2} ({})",
        frustration.expected(),
        frustration.legend[frustration.nearest()]
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
