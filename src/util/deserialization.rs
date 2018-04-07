use hex;
use std::fmt;
use serde::{self, de, Deserializer, Deserialize};
use std::str::FromStr;

pub fn bytes_from_hex_string<'de, D>(d: D) -> Result<Vec<u8>, D::Error>
    where D: Deserializer<'de> {
    d.deserialize_str(BytesFromhexStringJsonVisitor)
}

struct BytesFromhexStringJsonVisitor;

impl<'de> de::Visitor<'de> for BytesFromhexStringJsonVisitor {
    type Value = Vec<u8>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result
    {
        write!(formatter, "a hex string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where E: de::Error
    {
        hex::FromHex::from_hex(v).map_err(serde::de::Error::custom)
    }

}

pub fn from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: FromStr,
        T::Err: fmt::Display,
        D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(de::Error::custom)
}