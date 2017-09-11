use std::cmp::{Eq, Ordering};
use std::collections::BTreeMap;
use std::fmt;
use std::hash::{Hash, Hasher};

use serde::de::{Error, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};

use de;

/// A wrapper for `f64` which guarantees that the inner value
/// is finite and thus implements `Eq`, `Hash` and `Ord`.
#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
pub struct Number(f64);

impl Number {
    /// Panics if `v` is not a real number
    /// (infinity, NaN, ..).
    pub fn new(v: f64) -> Self {
        if !v.is_finite() {
            panic!("Tried to create Number with a NaN / infinity");
        }

        Number(v)
    }

    /// Returns the wrapped float.
    pub fn get(&self) -> f64 {
        self.0
    }
}

impl Eq for Number {}

impl Hash for Number {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0 as u64);
    }
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).expect("Bug: Contract violation")
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Value {
    Bool(bool),
    Char(char),
    Map(BTreeMap<Value, Value>),
    Number(Number),
    Option(Option<Box<Value>>),
    String(String),
    Seq(Vec<Value>),
    Unit,
}

impl Value {
    pub fn from_str(s: &str) -> de::Result<Self> {
        Self::deserialize(&mut super::Deserializer::from_str(s))
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "A RON value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
        where E: Error
    {
        Ok(Value::Bool(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where E: Error,
    {
        self.visit_f64(v as f64)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where E: Error
    {
        self.visit_f64(v as f64)
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where E: Error
    {
        Ok(Value::Number(Number::new(v)))
    }

    fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
        where E: Error
    {
        Ok(Value::Char(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where E: Error
    {
        self.visit_string(v.to_owned())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where E: Error
    {
        Ok(Value::String(v))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where E: Error
    {
        self.visit_byte_buf(v.to_vec())
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
        where E: Error
    {
        self.visit_string(String::from_utf8(v)
            .map_err(|e| Error::custom(format!("{}", e)))?)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
        where E: Error
    {
        Ok(Value::Option(None))
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where D: Deserializer<'de>,
    {
        Ok(Value::Option(Some(Box::new(deserializer.deserialize_any(ValueVisitor)?))))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
        where E: Error
    {
        Ok(Value::Unit)
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where D: Deserializer<'de>
    {
        deserializer.deserialize_any(ValueVisitor)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where A: SeqAccess<'de>
    {
        let mut vec = Vec::new();
        if let Some(cap) = seq.size_hint() {
            vec.reserve_exact(cap);
        }

        while let Some(x) = seq.next_element()? {
            vec.push(x);
        }

        Ok(Value::Seq(vec))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where A: MapAccess<'de>
    {
        let mut res: BTreeMap<Value, Value> = BTreeMap::new();

        while let Some(entry) = map.next_entry()? {
            res.insert(entry.0, entry.1);
        }

        Ok(Value::Map(res))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(s: &str) -> Value {
        Value::from_str(s).expect("Failed to parse")
    }

    #[test]
    fn test_none() {
        assert_eq!(eval("None"), Value::Option(None));
    }
}
