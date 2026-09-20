use jevrs_derive::Questions;

#[derive(Questions)]
struct Triage {
    #[jev(noul = "Is this urgent?", yes = "Urgent")]
    urgent: jevrs::Noul,
}

fn main() {}
