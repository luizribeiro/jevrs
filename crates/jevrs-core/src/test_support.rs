use crate::{ArrayMap, Indexed, Levels, Options};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Dept {
    Billing,
    Technical,
    Sales,
}

impl Indexed for Dept {
    fn all() -> &'static [Self] {
        &[Self::Billing, Self::Technical, Self::Sales]
    }

    fn index(self) -> usize {
        self as usize
    }
}

impl Options for Dept {
    const N: usize = 3;
    type Map<T: 'static> = ArrayMap<Self, T, 3>;

    fn key(self) -> &'static str {
        match self {
            Self::Billing => "billing",
            Self::Technical => "technical",
            Self::Sales => "sales",
        }
    }

    fn description(self) -> Option<&'static str> {
        match self {
            Self::Billing => Some("Payments, invoicing, refunds"),
            Self::Technical => Some("Bugs, outages, integrations"),
            Self::Sales => None,
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        Self::all()
            .iter()
            .copied()
            .find(|option| option.key() == key)
    }

    fn map_from_fn<T: 'static>(mut f: impl FnMut(Self) -> T) -> Self::Map<T> {
        ArrayMap::new([f(Self::Billing), f(Self::Technical), f(Self::Sales)])
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Frustration {
    Calm,
    Frustrated,
    VeryAngry,
}

impl Indexed for Frustration {
    fn all() -> &'static [Self] {
        &[Self::Calm, Self::Frustrated, Self::VeryAngry]
    }

    fn index(self) -> usize {
        self as usize
    }
}

impl Levels for Frustration {
    const N: usize = 3;
    type Map<T: 'static> = ArrayMap<Self, T, 3>;

    fn description(self) -> &'static str {
        match self {
            Self::Calm => "Calm",
            Self::Frustrated => "Frustrated",
            Self::VeryAngry => "Very angry",
        }
    }

    fn from_index(index: usize) -> Option<Self> {
        Self::all().get(index).copied()
    }

    fn map_from_fn<T: 'static>(mut f: impl FnMut(Self) -> T) -> Self::Map<T> {
        ArrayMap::new([f(Self::Calm), f(Self::Frustrated), f(Self::VeryAngry)])
    }
}
