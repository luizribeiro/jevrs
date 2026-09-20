use jevrs_derive::Questions;

#[derive(Questions)]
struct Triage {
    #[jev(noul = "Is this urgent?", choice = "Where should this go?")]
    urgent: jevrs::Noul,
}

fn main() {}
