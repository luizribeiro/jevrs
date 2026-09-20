//! Demonstrates runtime-defined triage questions with the native client.
//! Run with `cargo run -p native-dynamic` and `TYPESAFE_API_KEY` set.

use jevrs::{Client, DynLevels, DynOptions, Questions};

const STATE: &str = "Help! My payouts have been failing for 3 days.";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let client = Client::reqwest().from_env()?.build()?;
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
