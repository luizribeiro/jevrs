use alloc::{string::String, vec::Vec};
use core::{
    fmt,
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use crate::Error;

pub(crate) const OPTIONS_TOO_FEW: &str = "options must contain at least one entry";
pub(crate) const OPTIONS_TOO_MANY: &str = "options must contain at most 255 entries";
pub(crate) const OPTION_KEY_EMPTY: &str = "option keys must not be empty";
pub(crate) const OPTION_KEY_DUPLICATE: &str = "option keys must be unique";
pub(crate) const LEVELS_TOO_FEW: &str = "levels must contain at least two entries";
pub(crate) const LEVELS_TOO_MANY: &str = "levels must contain at most 10 entries";

/// A finite set whose values have a dense canonical order.
///
/// Implement this shared contract when implementing [`Options`] or [`Levels`]
/// by hand. Derives implement it automatically. Each value in
/// [`Indexed::all`] must occupy the position returned by [`Indexed::index`].
pub trait Indexed: Copy + Eq + Send + Sync + 'static {
    /// Returns every value in the order used for maps and wire criteria.
    fn all() -> &'static [Self];

    /// Returns this value's position for indexing its associated map.
    fn index(self) -> usize;
}

/// A statically defined set of named choices.
///
/// Implement this trait when the choices are known at compile time. The order
/// returned by [`Indexed::all`] defines their indexes and wire order.
///
/// ```
/// use jevrs_core::{ArrayMap, Indexed, Options};
///
/// #[derive(Clone, Copy, Eq, PartialEq)]
/// enum Department { Billing, Technical, Sales }
///
/// impl Indexed for Department {
///     fn all() -> &'static [Self] {
///         &[Self::Billing, Self::Technical, Self::Sales]
///     }
///     fn index(self) -> usize { self as usize }
/// }
///
/// impl Options for Department {
///     const N: usize = 3;
///     type Map<T: Send + Sync + 'static> = ArrayMap<Self, T, 3>;
///     fn key(self) -> &'static str {
///         match self {
///             Self::Billing => "billing",
///             Self::Technical => "technical",
///             Self::Sales => "sales",
///         }
///     }
///     fn description(self) -> Option<&'static str> { None }
///     fn from_key(key: &str) -> Option<Self> {
///         Self::all().iter().copied().find(|option| option.key() == key)
///     }
///     fn map_from_fn<T: Send + Sync + 'static>(f: impl FnMut(Self) -> T) -> Self::Map<T> {
///         let mut f = f;
///         ArrayMap::new([f(Self::Billing), f(Self::Technical), f(Self::Sales)])
///     }
/// }
/// ```
pub trait Options: Indexed {
    /// Declares the choice count so invalid derived or manual sets fail early.
    const N: usize;

    /// Forces a compile-time check of the API's 1-to-255 choice limit.
    const COUNT_OK: () = assert!(Self::N >= 1 && Self::N <= 255);

    /// Selects dense storage for one value per choice.
    type Map<T: Send + Sync + 'static>: Index<Self, Output = T>
        + IndexMut<Self>
        + Send
        + Sync
        + 'static;

    /// Returns the stable wire key used in criteria and response probabilities.
    fn key(self) -> &'static str;

    /// Returns guidance for the model, or `None` when the key is sufficient.
    fn description(self) -> Option<&'static str>;

    /// Converts the model's selected wire key back into the enum.
    fn from_key(key: &str) -> Option<Self>;

    /// Builds the associated map when decoding one value per choice.
    fn map_from_fn<T: Send + Sync + 'static>(f: impl FnMut(Self) -> T) -> Self::Map<T>;
}

/// A statically defined ordered scoring scale.
///
/// Implement this trait when level count and order are compile-time schema.
/// Use [`DynLevels`] when a scale comes from configuration or a database.
pub trait Levels: Indexed + Ord {
    /// Declares the scale length so invalid sets fail early.
    const N: usize;

    /// Forces a compile-time check of the API's 2-to-10 level limit.
    const COUNT_OK: () = assert!(Self::N >= 2 && Self::N <= 10);

    /// Selects dense storage for one value per level.
    type Map<T: Send + Sync + 'static>: Index<Self, Output = T>
        + IndexMut<Self>
        + Send
        + Sync
        + 'static;

    /// Returns the model guidance sent for this level.
    fn description(self) -> &'static str;

    /// Converts a zero-based wire index back into the level enum.
    fn from_index(index: usize) -> Option<Self>;

    /// Builds the associated map when decoding one value per level.
    fn map_from_fn<T: Send + Sync + 'static>(f: impl FnMut(Self) -> T) -> Self::Map<T>;
}

/// A runtime-defined set of named choices.
///
/// Use this with [`Questions::choice_dyn`](crate::Questions::choice_dyn) when
/// keys come from configuration or a database. Use [`Options`] for static
/// criteria and [`crate::ChoiceAnswer`] for its typed result.
#[derive(Clone, Debug, PartialEq)]
pub struct DynOptions {
    keys: Vec<String>,
    descriptions: Vec<Option<String>>,
}

impl DynOptions {
    /// Validates runtime choices before they reach a request builder.
    ///
    /// ```
    /// use jevrs_core::DynOptions;
    ///
    /// let options = DynOptions::new([
    ///     ("billing", Some("Payments and refunds")),
    ///     ("sales", None),
    /// ])?;
    /// assert_eq!(options.index_of("sales"), Some(1));
    /// # Ok::<(), jevrs_core::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCriteria`] when there are no options, there are
    /// more than 255, or a key is empty or duplicated.
    pub fn new<K, D, I>(options: I) -> Result<Self, Error>
    where
        K: Into<String>,
        D: Into<String>,
        I: IntoIterator<Item = (K, Option<D>)>,
    {
        let mut keys = Vec::new();
        let mut descriptions = Vec::new();
        for (key, description) in options {
            let key = key.into();
            if key.is_empty() {
                return Err(invalid_criteria(OPTION_KEY_EMPTY));
            }
            if keys.iter().any(|existing| existing == &key) {
                return Err(invalid_criteria(OPTION_KEY_DUPLICATE));
            }
            if keys.len() == 255 {
                return Err(invalid_criteria(OPTIONS_TOO_MANY));
            }
            keys.push(key);
            descriptions.push(description.map(Into::into));
        }
        if keys.is_empty() {
            return Err(invalid_criteria(OPTIONS_TOO_FEW));
        }
        Ok(Self { keys, descriptions })
    }

    /// Returns option keys in wire order for display or configuration checks.
    #[must_use]
    pub fn keys(&self) -> &[String] {
        &self.keys
    }

    /// Looks up model guidance when inspecting configured criteria.
    #[must_use]
    pub fn description(&self, key: &str) -> Option<&str> {
        self.index_of(key)
            .and_then(|index| self.descriptions[index].as_deref())
    }

    /// Finds a key's position when aligning external data with this set.
    #[must_use]
    pub fn index_of(&self, key: &str) -> Option<usize> {
        self.keys.iter().position(|candidate| candidate == key)
    }

    /// Returns the count when reporting or validating configured criteria.
    #[must_use]
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Reports whether the set has no options.
    ///
    /// A successfully constructed set is never empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

impl IntoIterator for DynOptions {
    type Item = (String, Option<String>);
    type IntoIter =
        core::iter::Zip<alloc::vec::IntoIter<String>, alloc::vec::IntoIter<Option<String>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.keys.into_iter().zip(self.descriptions)
    }
}

/// A runtime-defined ordered scoring scale.
///
/// Use this with [`Questions::score_dyn`](crate::Questions::score_dyn) when
/// level descriptions are runtime data. Use [`Levels`] for a static scale and
/// [`crate::ScoreAnswer`] for its typed result.
#[derive(Clone, Debug, PartialEq)]
pub struct DynLevels(Vec<String>);

impl DynLevels {
    /// Validates a runtime scale while preserving ascending score order.
    ///
    /// ```
    /// use jevrs_core::DynLevels;
    ///
    /// let levels = DynLevels::new(["Calm", "Frustrated", "Very angry"])?;
    /// assert_eq!(levels.levels()[1], "Frustrated");
    /// # Ok::<(), jevrs_core::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCriteria`] unless the scale contains between
    /// two and ten levels.
    pub fn new<S: Into<String>>(levels: impl IntoIterator<Item = S>) -> Result<Self, Error> {
        let mut levels = levels
            .into_iter()
            .map(Into::into)
            .take(11)
            .collect::<Vec<_>>();
        if levels.len() < 2 {
            return Err(invalid_criteria(LEVELS_TOO_FEW));
        }
        if levels.len() > 10 {
            return Err(invalid_criteria(LEVELS_TOO_MANY));
        }
        levels.shrink_to_fit();
        Ok(Self(levels))
    }

    /// Returns descriptions when displaying or inspecting the configured scale.
    #[must_use]
    pub fn levels(&self) -> &[String] {
        &self.0
    }

    /// Returns the count when reporting or validating a configured scale.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Reports whether the scale has no levels.
    ///
    /// A successfully constructed scale is never empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl IntoIterator for DynLevels {
    type Item = String;
    type IntoIter = alloc::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

fn invalid_criteria(reason: &'static str) -> Error {
    Error::InvalidCriteria { id: None, reason }
}

/// Dense per-key storage for a static [`Options`] or [`Levels`] implementation.
///
/// Use this as the `Map` associated type in a manual implementation. Derived
/// implementations choose it automatically.
#[derive(Clone, PartialEq)]
pub struct ArrayMap<K: Indexed, T, const N: usize>([T; N], PhantomData<K>);

impl<K, T, const N: usize> ArrayMap<K, T, N>
where
    K: Indexed,
{
    /// Creates storage when values are already arranged in [`Indexed`] order.
    pub const fn new(values: [T; N]) -> Self {
        Self(values, PhantomData)
    }

    /// Iterates in schema order when every key needs its decoded value.
    pub fn iter(&self) -> impl Iterator<Item = (K, &T)> {
        K::all().iter().copied().zip(self.0.iter())
    }

    /// Returns the array when a fixed-size consumer no longer needs typed keys.
    pub fn into_inner(self) -> [T; N] {
        self.0
    }
}

impl<K, T, const N: usize> Index<K> for ArrayMap<K, T, N>
where
    K: Indexed,
{
    type Output = T;

    fn index(&self, key: K) -> &Self::Output {
        &self.0[key.index()]
    }
}

impl<K, T, const N: usize> IndexMut<K> for ArrayMap<K, T, N>
where
    K: Indexed,
{
    fn index_mut(&mut self, key: K) -> &mut Self::Output {
        &mut self.0[key.index()]
    }
}

impl<K, T, const N: usize> fmt::Debug for ArrayMap<K, T, N>
where
    K: Indexed + fmt::Debug,
    T: fmt::Debug,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_map().entries(self.iter()).finish()
    }
}

#[cfg(test)]
mod tests {
    use alloc::{
        string::{String, ToString},
        vec,
        vec::Vec,
    };

    use super::{
        DynLevels, DynOptions, LEVELS_TOO_FEW, LEVELS_TOO_MANY, OPTION_KEY_DUPLICATE,
        OPTION_KEY_EMPTY, OPTIONS_TOO_FEW, OPTIONS_TOO_MANY,
    };
    use crate::{
        Error, Indexed, Levels, Options,
        test_support::{Dept, Frustration},
    };

    #[test]
    fn option_lookups_round_trip_and_reject_unknown_keys() {
        for option in Dept::all() {
            assert_eq!(Dept::from_key(option.key()), Some(*option));
            assert_eq!(Dept::all().get(option.index()), Some(option));
        }
        assert_eq!(Dept::from_key("support"), None);
    }

    #[test]
    fn level_lookups_round_trip_and_reject_unknown_indexes() {
        for level in Frustration::all() {
            assert_eq!(Frustration::from_index(level.index()), Some(*level));
        }
        assert_eq!(Frustration::from_index(3), None);
    }

    #[test]
    fn array_map_indexes_mutates_and_unwraps() {
        let mut map = Dept::map_from_fn(Dept::index);
        assert_eq!(map[Dept::Technical], 1);
        map[Dept::Sales] = 9;
        assert_eq!(map.into_inner(), [0, 1, 9]);
    }

    #[test]
    fn array_map_iterates_in_key_order() {
        let map = Frustration::map_from_fn(Frustration::description);
        let entries: alloc::vec::Vec<_> = map.iter().collect();
        assert_eq!(
            entries,
            alloc::vec![
                (Frustration::Calm, &"Calm"),
                (Frustration::Frustrated, &"Frustrated"),
                (Frustration::VeryAngry, &"Very angry"),
            ]
        );
    }

    fn invalid_reason(error: Error) -> &'static str {
        match error {
            Error::InvalidCriteria { id: None, reason } => reason,
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn dynamic_options_reject_every_invalid_shape() {
        let empty: Vec<(String, Option<String>)> = Vec::new();
        assert_eq!(
            invalid_reason(DynOptions::new(empty).unwrap_err()),
            OPTIONS_TOO_FEW
        );
        let too_many = (0..256)
            .map(|index| (index.to_string(), None::<String>))
            .collect::<Vec<_>>();
        assert_eq!(
            invalid_reason(DynOptions::new(too_many).unwrap_err()),
            OPTIONS_TOO_MANY
        );
        assert_eq!(
            invalid_reason(
                DynOptions::new([("billing", None::<&str>), ("billing", None)]).unwrap_err()
            ),
            OPTION_KEY_DUPLICATE
        );
        assert_eq!(
            invalid_reason(DynOptions::new([(String::new(), None::<String>)]).unwrap_err()),
            OPTION_KEY_EMPTY
        );
    }

    #[test]
    fn dynamic_options_accept_borrowed_and_owned_strings() {
        let borrowed =
            DynOptions::new([("billing", Some("Payments and refunds")), ("sales", None)]).unwrap();
        assert_eq!(
            borrowed.description("billing"),
            Some("Payments and refunds")
        );

        let owned = DynOptions::new([(
            "technical".to_string(),
            Some("Bugs and outages".to_string()),
        )])
        .unwrap();
        assert_eq!(owned.keys(), &["technical"]);

        let entries: Vec<(String, Option<String>)> =
            vec![("sales".to_string(), Some("New accounts".to_string()))];
        let from_vec = DynOptions::new(entries).unwrap();
        assert_eq!(from_vec.description("sales"), Some("New accounts"));
    }

    #[test]
    fn dynamic_options_accept_255_entries_and_expose_accessors() {
        let entries = (0..255)
            .map(|index| {
                let key = index.to_string();
                let description = (index == 1).then(|| "Technical support".to_string());
                (key, description)
            })
            .collect::<Vec<_>>();
        let options = DynOptions::new(entries).unwrap();

        assert_eq!(options.len(), 255);
        assert_eq!(options.keys().first().map(String::as_str), Some("0"));
        assert_eq!(options.keys().last().map(String::as_str), Some("254"));
        assert_eq!(options.description("1"), Some("Technical support"));
        assert_eq!(options.description("2"), None);
        assert_eq!(options.description("missing"), None);
        assert_eq!(options.index_of("254"), Some(254));
        assert_eq!(options.index_of("missing"), None);
    }

    #[test]
    fn dynamic_levels_validate_bounds_and_expose_accessors() {
        assert_eq!(
            invalid_reason(DynLevels::new(["Calm"]).unwrap_err()),
            LEVELS_TOO_FEW
        );
        assert_eq!(
            invalid_reason(DynLevels::new((0..11).map(|index| index.to_string())).unwrap_err()),
            LEVELS_TOO_MANY
        );

        let levels = DynLevels::new((0..10).map(|index| index.to_string())).unwrap();
        assert_eq!(levels.len(), 10);
        assert_eq!(levels.levels().first().map(String::as_str), Some("0"));
        assert_eq!(levels.levels().last().map(String::as_str), Some("9"));
    }

    #[test]
    fn dynamic_levels_accept_borrowed_and_owned_strings() {
        let borrowed = DynLevels::new(["Calm", "Frustrated"]).unwrap();
        assert_eq!(borrowed.levels(), &["Calm", "Frustrated"]);

        let owned = DynLevels::new(["Calm".to_string(), "Frustrated".to_string()]).unwrap();
        assert_eq!(owned.levels(), &["Calm", "Frustrated"]);
    }
}
