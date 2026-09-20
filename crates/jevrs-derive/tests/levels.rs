#![allow(missing_docs)]

use jevrs::{Indexed, Levels};
use jevrs_derive::Levels;

#[derive(Clone, Copy, Debug, Eq, Levels, Ord, PartialEq, PartialOrd)]
enum Frustration {
    /// Calm
    Calm,
    /// Frustrated
    Frustrated,
    /// Very
    /// angry
    VeryAngry,
}

#[derive(Clone, Copy, Debug, Eq, Levels, Ord, PartialEq, PartialOrd)]
enum ExplicitLevels {
    /// Low
    Low = 5,
    /// Medium
    Medium,
    /// High
    High = 12,
}

#[test]
fn derives_the_triage_levels() {
    assert_eq!(Frustration::N, 3);
    assert_eq!(
        Frustration::all(),
        &[
            Frustration::Calm,
            Frustration::Frustrated,
            Frustration::VeryAngry
        ]
    );
    assert_eq!(Frustration::Calm.description(), "Calm");
    assert_eq!(Frustration::Frustrated.description(), "Frustrated");
    assert_eq!(Frustration::VeryAngry.description(), "Very angry");
    assert_eq!(Frustration::Calm.index(), 0);
    assert_eq!(Frustration::Frustrated.index(), 1);
    assert_eq!(Frustration::VeryAngry.index(), 2);
    assert_eq!(Frustration::from_index(0), Some(Frustration::Calm));
    assert_eq!(Frustration::from_index(1), Some(Frustration::Frustrated));
    assert_eq!(Frustration::from_index(2), Some(Frustration::VeryAngry));
    assert_eq!(Frustration::from_index(3), None);

    let map = Frustration::map_from_fn(Indexed::index);
    assert_eq!(map.into_inner(), [0, 1, 2]);
}

#[test]
fn explicit_discriminants_keep_positional_indexes() {
    assert_eq!(ExplicitLevels::Low.index(), 0);
    assert_eq!(ExplicitLevels::Medium.index(), 1);
    assert_eq!(ExplicitLevels::High.index(), 2);
    assert_eq!(ExplicitLevels::from_index(1), Some(ExplicitLevels::Medium));
}

mod core_path {
    use jevrs_core::{Indexed, Levels};
    use jevrs_derive::Levels;

    #[derive(Clone, Copy, Debug, Eq, Levels, Ord, PartialEq, PartialOrd)]
    #[jev(crate = "::jevrs_core")]
    enum CoreFrustration {
        /// Calm
        Calm,
        /// Angry
        Angry,
    }

    #[test]
    fn overrides_the_emitted_crate_path() {
        assert_eq!(
            CoreFrustration::all(),
            &[CoreFrustration::Calm, CoreFrustration::Angry]
        );
        assert_eq!(CoreFrustration::Angry.description(), "Angry");
        assert_eq!(CoreFrustration::from_index(0), Some(CoreFrustration::Calm));
    }
}
