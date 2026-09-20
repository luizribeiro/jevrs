use jevrs_derive::Options;

#[derive(Options)]
enum Dept {
    Billing,
    #[jev(key = "billing")]
    Technical,
}

fn main() {}
