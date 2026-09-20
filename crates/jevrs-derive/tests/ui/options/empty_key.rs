use jevrs_derive::Options;

#[derive(Options)]
enum Dept {
    #[jev(key = "")]
    Billing,
}

fn main() {}
