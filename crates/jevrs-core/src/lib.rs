#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]
//! Sans-I/O types, validation, and JSON codecs for the Jev API.

extern crate alloc;

/// The version of this crate.
///
/// ```
/// assert_eq!(jevrs_core::CRATE_VERSION, "0.0.0");
/// ```
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::CRATE_VERSION;

    #[test]
    fn crate_version_matches_workspace_version() {
        assert_eq!(CRATE_VERSION, "0.0.0");
    }
}
