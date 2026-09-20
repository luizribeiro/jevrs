use jevrs_derive::Questions;

#[derive(Questions)]
struct Triage {
    #[jev(choice = "Where should this go?", yes = "Billing", no = "Sales")]
    department: jevrs::Noul,
}

fn main() {}
