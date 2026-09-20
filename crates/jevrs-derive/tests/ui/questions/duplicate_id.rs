use jevrs_derive::Questions;

#[derive(Questions)]
struct Triage {
    #[jev(noul = "Is this urgent?")]
    urgent: jevrs::Noul,
    #[jev(noul = "Is this severe?", id = "urgent")]
    severe: jevrs::Noul,
}

fn main() {}
