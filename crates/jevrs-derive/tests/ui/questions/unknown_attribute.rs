use jevrs_derive::Questions;

#[derive(Questions)]
struct Triage {
    #[jev(noul = "Is this urgent?", label = "urgent")]
    urgent: jevrs::Noul,
}

fn main() {}
