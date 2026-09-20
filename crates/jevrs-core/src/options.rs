use core::{
    fmt,
    marker::PhantomData,
    ops::{Index, IndexMut},
};

/// A finite set whose values have a dense canonical order.
///
/// Derive implementations use this shared contract for both [`Options`] and
/// [`Levels`]. Each value in [`Indexed::all`] must occupy the position returned
/// by [`Indexed::index`].
pub trait Indexed: Copy + Eq + 'static {
    /// Returns every value in canonical index order.
    fn all() -> &'static [Self];

    /// Returns this value's position in [`Indexed::all`].
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
///     type Map<T: 'static> = ArrayMap<Self, T, 3>;
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
///     fn map_from_fn<T: 'static>(f: impl FnMut(Self) -> T) -> Self::Map<T> {
///         let mut f = f;
///         ArrayMap::new([f(Self::Billing), f(Self::Technical), f(Self::Sales)])
///     }
/// }
/// ```
pub trait Options: Indexed {
    /// The number of choices in the set.
    const N: usize;

    /// Fails compilation when [`Options::N`] is outside the API bounds.
    const COUNT_OK: () = assert!(Self::N >= 1 && Self::N <= 255);

    /// Dense storage containing one value per choice.
    type Map<T: 'static>: Index<Self, Output = T> + IndexMut<Self> + 'static;

    /// Returns the choice's wire key.
    fn key(self) -> &'static str;

    /// Returns the optional choice description sent to the API.
    fn description(self) -> Option<&'static str>;

    /// Finds a choice by its wire key.
    fn from_key(key: &str) -> Option<Self>;

    /// Builds dense storage by calling `f` once per choice in index order.
    fn map_from_fn<T: 'static>(f: impl FnMut(Self) -> T) -> Self::Map<T>;
}

/// A statically defined ordered scoring scale.
pub trait Levels: Indexed + Ord {
    /// The number of levels in the scale.
    const N: usize;

    /// Fails compilation when [`Levels::N`] is outside the API bounds.
    const COUNT_OK: () = assert!(Self::N >= 2 && Self::N <= 10);

    /// Dense storage containing one value per level.
    type Map<T: 'static>: Index<Self, Output = T> + IndexMut<Self> + 'static;

    /// Returns the human-readable level description sent to the API.
    fn description(self) -> &'static str;

    /// Finds a level by its numeric index.
    fn from_index(index: usize) -> Option<Self>;

    /// Builds dense storage by calling `f` once per level in index order.
    fn map_from_fn<T: 'static>(f: impl FnMut(Self) -> T) -> Self::Map<T>;
}

/// Dense per-key storage for a static [`Options`] or [`Levels`] implementation.
#[derive(Clone, PartialEq)]
pub struct ArrayMap<K: Indexed, T, const N: usize>([T; N], PhantomData<K>);

impl<K, T, const N: usize> ArrayMap<K, T, N>
where
    K: Indexed,
{
    /// Creates a map from values arranged in key order.
    pub const fn new(values: [T; N]) -> Self {
        Self(values, PhantomData)
    }

    /// Iterates over key-value pairs in index order.
    pub fn iter(&self) -> impl Iterator<Item = (K, &T)> {
        K::all().iter().copied().zip(self.0.iter())
    }

    /// Returns the underlying fixed-size array.
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
    use crate::{
        Indexed, Levels, Options,
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
}
