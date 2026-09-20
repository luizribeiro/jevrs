use core::fmt;

use serde::{Deserialize, Deserializer, Serialize};

macro_rules! unit_interval {
    ($name:ident, $docs:literal) => {
        #[doc = $docs]
        #[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize)]
        #[repr(transparent)]
        pub struct $name(f64);

        impl $name {
            /// Validates an untrusted floating-point value at an input boundary.
            #[must_use]
            pub const fn new(value: f64) -> Option<Self> {
                if value >= 0.0 && value <= 1.0 {
                    Some(Self(value))
                } else {
                    None
                }
            }

            /// Returns the scalar for arithmetic, formatting, or comparison.
            #[must_use]
            pub const fn get(self) -> f64 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = f64::deserialize(deserializer)?;
                Self::new(value).ok_or_else(|| {
                    serde::de::Error::custom(format_args!(
                        "invalid {} value {value}; expected a number from 0 to 1",
                        stringify!($name)
                    ))
                })
            }
        }
    };
}

unit_interval!(
    Probability,
    "A probability constrained to the inclusive range from zero to one.\n\n\
     Use [`Probability::new`] at input boundaries to reject invalid values.\n\n\
     ```\n\
     let probability = jevrs_core::Probability::new(0.75).unwrap();\n\
     assert_eq!(probability.get(), 0.75);\n\
     assert!(jevrs_core::Probability::new(1.01).is_none());\n\
     ```"
);

unit_interval!(
    Confidence,
    "A model confidence constrained to the inclusive range from zero to one.\n\n\
     Use this when comparing or displaying the confidence on a \
     [`ChoiceAnswer`](crate::ChoiceAnswer), [`ScoreAnswer`](crate::ScoreAnswer), \
     [`DynChoiceAnswer`](crate::DynChoiceAnswer), or \
     [`DynScoreAnswer`](crate::DynScoreAnswer)."
);

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::{Confidence, Probability};

    #[test]
    fn accepts_both_bounds() {
        assert_eq!(Probability::new(0.0).map(Probability::get), Some(0.0));
        assert_eq!(Probability::new(1.0).map(Probability::get), Some(1.0));
        assert_eq!(Confidence::new(0.0).map(Confidence::get), Some(0.0));
        assert_eq!(Confidence::new(1.0).map(Confidence::get), Some(1.0));
    }

    #[test]
    fn rejects_values_outside_bounds_and_nan() {
        assert!(Probability::new(-f64::EPSILON).is_none());
        assert!(Probability::new(1.0 + f64::EPSILON).is_none());
        assert!(Probability::new(f64::NAN).is_none());
        assert!(Confidence::new(-f64::EPSILON).is_none());
        assert!(Confidence::new(1.0 + f64::EPSILON).is_none());
        assert!(Confidence::new(f64::NAN).is_none());
    }

    #[test]
    fn serializes_as_a_number_and_round_trips() {
        let probability = Probability::new(0.79).unwrap();
        let encoded = serde_json::to_string(&probability).unwrap();
        assert_eq!(encoded, "0.79");
        assert_eq!(
            serde_json::from_str::<Probability>(&encoded).unwrap(),
            probability
        );

        let confidence = Confidence::new(0.93).unwrap();
        let encoded = serde_json::to_string(&confidence).unwrap();
        assert_eq!(encoded, "0.93");
        assert_eq!(
            serde_json::from_str::<Confidence>(&encoded).unwrap(),
            confidence
        );
    }

    #[test]
    fn deserialization_errors_name_the_invalid_value() {
        let too_high = serde_json::from_str::<Probability>("1.5")
            .unwrap_err()
            .to_string();
        assert!(too_high.contains("1.5"));

        let too_low = serde_json::from_str::<Confidence>("-0.01")
            .unwrap_err()
            .to_string();
        assert!(too_low.contains("-0.01"));
    }
}
