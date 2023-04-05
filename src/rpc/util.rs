use std::fmt;
use serde::{Deserializer, de::{Visitor, Unexpected, Error}};

/// Small utility function to deserialize a string into a f64.
pub(crate) fn string_as_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(F64Visitor)
}

struct F64Visitor;
impl<'de> Visitor<'de> for F64Visitor {
    type Value = f64;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a string representation of a f64")
    }
    fn visit_str<E>(self, value: &str) -> Result<f64, E>
    where
        E: Error,
    {
        value.parse::<f64>().map_err(|_err| {
            E::invalid_value(Unexpected::Str(value), &"a string representation of a f64")
        })
    }
}