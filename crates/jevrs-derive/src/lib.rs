//! Derive macros for strongly typed Jev criteria.

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod attrs;
mod indexed;
mod levels;
mod options;

/// Derives `jevrs::Options` for a unit-variant enum.
///
/// Variant names become `snake_case` wire keys. Attributes customize the
/// generated implementation:
///
/// | Location | Attribute | Purpose |
/// | --- | --- | --- |
/// | Enum | `#[jev(crate = "path")]` | Override the default `::jevrs` path. |
/// | Variant | `#[jev(key = "name")]` | Override the wire key. |
/// | Variant | `#[jev(desc = "text")]` | Override the option description. |
///
/// A variant's trimmed `///` lines supply its description when `desc` is
/// absent. A variant without either has no description.
///
/// ```
/// use jevrs::{Indexed, Options};
/// use jevrs_derive::Options;
///
/// #[derive(Clone, Copy, Debug, Eq, Options, PartialEq)]
/// enum Dept {
///     /// Payments, invoicing, refunds
///     Billing,
///     #[jev(key = "tech", desc = "Bugs, outages, integrations")]
///     Technical,
///     Sales,
/// }
///
/// assert_eq!(Dept::all(), &[Dept::Billing, Dept::Technical, Dept::Sales]);
/// assert_eq!(Dept::Technical.key(), "tech");
/// assert_eq!(Dept::Sales.description(), None);
/// ```
///
/// Compilation fails for non-enums, empty enums, non-unit variants, empty or
/// duplicate wire keys, unknown `jev` attributes, or more than 255 variants.
#[proc_macro_derive(Options, attributes(jev))]
pub fn derive_options(input: TokenStream) -> TokenStream {
    options::expand(&parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derives `jevrs::Levels` for an ordered unit-variant enum.
///
/// Variant order defines level order. Attributes customize the generated
/// implementation:
///
/// | Location | Attribute | Purpose |
/// | --- | --- | --- |
/// | Enum | `#[jev(crate = "path")]` | Override the default `::jevrs` path. |
/// | Variant | `#[jev(desc = "text")]` | Override the level description. |
///
/// Every variant needs either `desc` or at least one `///` doc line. Trimmed
/// doc lines are joined with one space.
///
/// ```
/// use jevrs::{Indexed, Levels};
/// use jevrs_derive::Levels;
///
/// #[derive(Clone, Copy, Debug, Eq, Levels, Ord, PartialEq, PartialOrd)]
/// enum Frustration {
///     /// Calm
///     Calm,
///     /// Frustrated
///     Frustrated,
///     #[jev(desc = "Very angry")]
///     VeryAngry,
/// }
///
/// assert_eq!(Frustration::all()[2], Frustration::VeryAngry);
/// assert_eq!(Frustration::VeryAngry.description(), "Very angry");
/// ```
///
/// Compilation fails for non-enums, non-unit variants, fewer than two or more
/// than ten variants, missing descriptions, `key` attributes, or unknown
/// `jev` attributes.
#[proc_macro_derive(Levels, attributes(jev))]
pub fn derive_levels(input: TokenStream) -> TokenStream {
    levels::expand(&parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
