use crate::{ArrayMap, Indexed, Levels, Options};

pub(crate) const TRIAGE_RESPONSE: &[u8] = br#"{
  "model": "jev-1.13.0",
  "answers": {
    "is_urgent": { "type": "noul", "noul": 0.95 },
    "department": { "type": "choice", "choice": "billing", "confidence": 0.79,
      "probabilities": { "billing": 0.86, "technical": 0.14, "sales": 0.0 } },
    "frustration": { "type": "score", "score": 1.05, "confidence": 0.93,
      "legend": { "0": "Calm", "1": "Frustrated", "2": "Very angry" },
      "probabilities": { "0": 0.0, "1": 0.95, "2": 0.05 } }
  },
  "usage": { "input_tokens": 414, "output_tokens": 73 }
}"#;

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
    type Map<T: Send + Sync + 'static> = ArrayMap<Self, T, 3>;

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

    fn map_from_fn<T: Send + Sync + 'static>(mut f: impl FnMut(Self) -> T) -> Self::Map<T> {
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
    type Map<T: Send + Sync + 'static> = ArrayMap<Self, T, 3>;

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

    fn map_from_fn<T: Send + Sync + 'static>(mut f: impl FnMut(Self) -> T) -> Self::Map<T> {
        ArrayMap::new([f(Self::Calm), f(Self::Frustrated), f(Self::VeryAngry)])
    }
}
