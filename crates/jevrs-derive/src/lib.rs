//! Derive macros for strongly typed Jev criteria.

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod attrs;
mod indexed;
mod levels;
mod options;
mod questions;

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
/// use jevrs::{Indexed, Options as OptionsTrait};
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
/// use jevrs::{Indexed, Levels as LevelsTrait};
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

/// Derives a reusable, statically typed Jev question set from named fields.
///
/// Each field has exactly one question-kind attribute. The field type is its
/// question marker, and `id` defaults to the field name:
///
/// | Kind | Required attribute | Optional attributes |
/// | --- | --- | --- |
/// | Yes/no | `#[jev(noul = "instructions")]` | `id`, or both `yes` and `no` |
/// | Choice | `#[jev(choice = "instructions")]` | `id` |
/// | Score | `#[jev(score = "instructions")]` | `id` |
///
/// Set `#[jev(crate = "::jevrs_core")]` on the struct when using the core
/// crate directly. The derive generates public `<Name>Handles` and
/// `<Name>Answers` structs.
///
/// ```
/// use jevrs::{Choice, Noul, QuestionSet, Score};
/// use jevrs_derive::{Levels, Options, Questions};
///
/// #[derive(Clone, Copy, Debug, Eq, Options, PartialEq)]
/// enum Dept {
///     /// Payments and refunds
///     Billing,
///     Sales,
/// }
/// #[derive(Clone, Copy, Debug, Eq, Levels, Ord, PartialEq, PartialOrd)]
/// enum Mood {
///     /// Calm
///     Calm,
///     /// Frustrated
///     Frustrated,
/// }
/// #[derive(Questions)]
/// struct Triage {
///     #[jev(noul = "Does this convey urgency?")]
///     urgent: Noul,
///     #[jev(choice = "Which team should handle this?")]
///     department: Choice<Dept>,
///     #[jev(score = "How frustrated is the customer?", id = "mood")]
///     frustration: Score<Mood>,
/// }
///
/// let (questions, handles) = Triage::questions()?;
/// assert_eq!(questions.len(), 3);
/// let _ = handles;
/// # Ok::<(), jevrs::Error>(())
/// ```
///
/// Compilation fails for non-structs, tuple or empty structs, missing or
/// conflicting question kinds, incomplete yes/no criteria, unknown attributes,
/// or duplicate wire IDs.
#[proc_macro_derive(Questions, attributes(jev))]
pub fn derive_questions(input: TokenStream) -> TokenStream {
    questions::expand(&parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
