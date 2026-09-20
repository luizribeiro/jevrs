use jevrs_derive::Options;

#[derive(Options)]
enum Dept {
    Billing(String),
}

fn main() {}
