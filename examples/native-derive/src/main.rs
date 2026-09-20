//! Shows the typed triage flow with derives and the native client.
//!
//! Set `TYPESAFE_API_KEY`, then run:
//!
//! ```text
//! cargo run -p native-derive
//! ```

use jevrs::{Choice, Client, Levels, Noul, Options, Questions, Score};

const STATE: &str = "Help! My payouts have been failing for 3 days.";

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::reqwest().from_env()?.build()?;
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
