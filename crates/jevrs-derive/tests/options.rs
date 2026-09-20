#![allow(missing_docs)]

use jevrs::{Indexed, Options as OptionsTrait};
use jevrs_derive::Options;

#[derive(Clone, Copy, Debug, Eq, Options, PartialEq)]
enum Dept {
    /// Payments, invoicing,
    /// refunds
    Billing,
    /// This doc is overridden.
    #[jev(key = "tech", desc = "Bugs, outages, integrations")]
    Technical,
    Sales,
}

#[derive(Clone, Copy, Debug, Eq, Options, PartialEq)]
enum ExplicitOptions {
    A = 5,
    B,
    C = 12,
}

#[test]
fn derives_the_triage_options() {
    assert_eq!(Dept::N, 3);
    assert_eq!(Dept::all(), &[Dept::Billing, Dept::Technical, Dept::Sales]);
    assert_eq!(Dept::Billing.key(), "billing");
    assert_eq!(Dept::Technical.key(), "tech");
    assert_eq!(Dept::Sales.key(), "sales");
    assert_eq!(
        Dept::Billing.description(),
        Some("Payments, invoicing, refunds")
    );
    assert_eq!(
        Dept::Technical.description(),
        Some("Bugs, outages, integrations")
    );
    assert_eq!(Dept::Sales.description(), None);
    assert_eq!(Dept::from_key("billing"), Some(Dept::Billing));
    assert_eq!(Dept::from_key("tech"), Some(Dept::Technical));
    assert_eq!(Dept::from_key("sales"), Some(Dept::Sales));
    assert_eq!(Dept::from_key("unknown"), None);
    assert_eq!(Dept::Billing.index(), 0);
    assert_eq!(Dept::Technical.index(), 1);
    assert_eq!(Dept::Sales.index(), 2);

    let map = Dept::map_from_fn(Indexed::index);
    assert_eq!(map.into_inner(), [0, 1, 2]);
}

#[test]
fn explicit_discriminants_keep_positional_indexes() {
    assert_eq!(ExplicitOptions::A.index(), 0);
    assert_eq!(ExplicitOptions::B.index(), 1);
    assert_eq!(ExplicitOptions::C.index(), 2);
    assert_eq!(ExplicitOptions::from_key("b"), Some(ExplicitOptions::B));
}

mod core_path {
    use jevrs_core::{Indexed, Options};
    use jevrs_derive::Options;

    #[derive(Clone, Copy, Debug, Eq, Options, PartialEq)]
    #[jev(crate = "::jevrs_core")]
    enum CoreDept {
        /// Billing
        Billing,
        /// Technical
        Technical,
    }

    #[test]
    fn overrides_the_emitted_crate_path() {
        assert_eq!(CoreDept::all(), &[CoreDept::Billing, CoreDept::Technical]);
        assert_eq!(CoreDept::Technical.key(), "technical");
        assert_eq!(CoreDept::from_key("billing"), Some(CoreDept::Billing));
    }
}
