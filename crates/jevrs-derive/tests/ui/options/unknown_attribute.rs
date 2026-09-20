use jevrs_derive::Options;

#[derive(Options)]
enum Dept {
    #[jev(label = "Billing")]
    Billing,
}

fn main() {}
