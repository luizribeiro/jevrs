use jevrs_derive::Levels;

#[derive(Levels)]
enum Frustration {
    /// Calm
    Calm,
    /// Angry
    #[jev(key = "angry")]
    Angry,
}

fn main() {}
