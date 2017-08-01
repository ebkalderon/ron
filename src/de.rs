use std::borrow::Cow;
use std::char::{decode_utf16, REPLACEMENT_CHARACTER};
use std::error::Error as StdError;
use std::fmt;
use std::str::FromStr;

use parse::Bytes;

use serde::de::{self, Deserializer as Deserializer_, DeserializeSeed, Visitor};

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Eof,
    Syntax,
    ExpectedArray,
    ExpectedArrayComma,
    ExpectedArrayEnd,
    ExpectedBoolean,
    ExpectedEnum,
    ExpectedChar,
    ExpectedFloat,
    ExpectedInteger,
    ExpectedOption,
    ExpectedOptionEnd,
    ExpectedMap,
    ExpectedMapColon,
    ExpectedMapComma,
    ExpectedMapEnd,
    ExpectedStruct,
    ExpectedStructEnd,
    ExpectedUnit,
    ExpectedStructName,
    ExpectedString,
    ExpectedIdentifier,

    InvalidEscape,

    /// A custom error emitted by the deserializer.
    Message(String),
    TrailingCharacters,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Message(ref e) => write!(f, "Custom message: {}", e),
            _ => unimplemented!()
        }
    }
}

impl de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Message(ref e) => e,
            _ => unimplemented!()
        }
    }
}

pub struct Deserializer<'de> {
    bytes: Bytes<'de>,
}

impl<'de> Deserializer<'de> {
    pub fn from_str(input: &'de str) -> Self {
        Deserializer {
            bytes: Bytes::new(input.as_bytes()),
        }
    }

    pub fn from_bytes(input: &'de [u8]) -> Self {
        Deserializer {
            bytes: Bytes::new(input),
        }
    }

    pub fn remainder(&self) -> Cow<str> {
        String::from_utf8_lossy(&self.bytes.bytes())
    }
}

pub fn from_str<'a, T>(s: &'a str) -> Result<T>
    where T: de::Deserialize<'a>
{
    let mut deserializer = Deserializer::from_str(s);
    let t = T::deserialize(&mut deserializer)?;

    deserializer.end()?;

    Ok(t)
}

impl<'de> Deserializer<'de> {
    /// Check if the remaining bytes are whitespace only,
    /// otherwise return an error.
    pub fn end(&mut self) -> Result<()> {
        self.bytes.skip_ws();

        if self.bytes.bytes().is_empty() {
            Ok(())
        } else {
            Err(Error::TrailingCharacters)
        }
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        panic!("Give me some!");
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_bool(self.bytes.bool()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_i8(self.bytes.signed_integer()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_i8(self.bytes.signed_integer()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_i32(self.bytes.signed_integer()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_i64(self.bytes.signed_integer()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_u8(self.bytes.unsigned_integer()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_u16(self.bytes.unsigned_integer()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_u32(self.bytes.unsigned_integer()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_u64(self.bytes.unsigned_integer()?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_f32(self.bytes.float()?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_f64(self.bytes.float()?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_char(self.bytes.char()?)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        let special_char = sym(b'\\') | sym(b'/') | sym(b'"')
            | sym(b'b').map(|_|b'\x08') | sym(b'f').map(|_|b'\x0C')
            | sym(b'n').map(|_|b'\n') | sym(b'r').map(|_|b'\r') | sym(b't').map(|_|b'\t');
        let escape_sequence = sym(b'\\') * special_char;
        let char_string = (none_of(b"\\\"") | escape_sequence).repeat(0..).convert(String::from_utf8);
        let utf16_char = seq(b"\\u") * is_a(char_class::hex_digit).repeat(4).convert(String::from_utf8).convert(|digits|u16::from_str_radix(&digits, 16));
        let utf16_string = utf16_char.repeat(0..).map(|chars| decode_utf16(chars).map(|r| r.unwrap_or(REPLACEMENT_CHARACTER)).collect::<String>());
        let parser = sym(b'"') * (char_string | utf16_string) - sym(b'"');

        match parser.parse(&mut self.input) {
            Ok(string) => visitor.visit_string(string),
            Err(_) => Err(Error::ExpectedString)
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        if self.bytes.consume("Some(") {
            let v = visitor.visit_some(&mut *self)?;

            if self.bytes.consume(")") {
                Ok(v)
            } else {
                Err(Error::ExpectedOptionEnd)
            }

        } else if self.bytes.consume("None") {
            visitor.visit_none()
        } else {
            Err(Error::ExpectedOption)
        }
    }

    // In Serde, unit means an anonymous value containing no data.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        if self.bytes.consume("()") {
            visitor.visit_unit()
        } else {
            Err(Error::ExpectedUnit)
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V
    ) -> Result<V::Value>
        where V: Visitor<'de>
    {
        if self.bytes.consume(name) {
            visitor.visit_unit()
        } else {
            self.deserialize_unit(visitor)
        }
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V
    ) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.bytes.consume(name);

        if self.bytes.consume("(") {
            let value = visitor.visit_newtype_struct(&mut *self)?;
            self.bytes.comma();

            if self.bytes.consume(")") {
                Ok(value)
            } else {
                Err(Error::ExpectedStructEnd)
            }
        } else {
            Err(Error::ExpectedStruct)
        }
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        if self.bytes.consume("[") {
            let value = visitor.visit_seq(CommaSeparated::new(b']', &mut self))?;
            self.bytes.comma();

            if self.bytes.consume("]") {
                Ok(value)
            } else {
                Err(Error::ExpectedArrayEnd)
            }
        } else {
            Err(Error::ExpectedArray)
        }
    }

    // Tuples look just like sequences in JSON. Some formats may be able to
    // represent tuples more efficiently.
    //
    // As indicated by the length parameter, the `Deserialize` implementation
    // for a tuple in the Serde data model is required to know the length of the
    // tuple before even looking at the input data.
    fn deserialize_tuple<V>(
        mut self,
        _len: usize,
        visitor: V
    ) -> Result<V::Value>
        where V: Visitor<'de>
    {
        if self.bytes.consume("(") {
            let value = visitor.visit_seq(CommaSeparated::new(b')', &mut self))?;
            self.bytes.comma();

            if self.bytes.consume(")") {
                Ok(value)
            } else {
                Err(Error::ExpectedArrayEnd)
            }
        } else {
            Err(Error::ExpectedArray)
        }
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V
    ) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.bytes.consume(name);
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        if self.bytes.consume("{") {
            let value = visitor.visit_map(CommaSeparated::new(b'}', &mut self))?;
            self.bytes.comma();

            if self.bytes.consume("}") {
                Ok(value)
            } else {
                Err(Error::ExpectedMapEnd)
            }
        } else {
            Err(Error::ExpectedMap)
        }
    }

    fn deserialize_struct<V>(
        mut self,
        name: &'static str,
        _fields: &'static [&'static str],
        visitor: V
    ) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.bytes.consume(name);

        if self.bytes.consume("(") {
            let value = visitor.visit_map(CommaSeparated::new(b')', &mut self))?;
            self.bytes.comma();

            if self.bytes.consume(")") {
                Ok(value)
            } else {
                Err(Error::ExpectedStructEnd)
            }
        } else {
            Err(Error::ExpectedStruct)
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V
    ) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_enum(Enum::new(self))
    }

    fn deserialize_identifier<V>(
        self,
        visitor: V
    ) -> Result<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_bytes(self.bytes.identifier()?)
    }

    fn deserialize_ignored_any<V>(
        self,
        visitor: V
    ) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.deserialize_any(visitor)
    }
}

struct CommaSeparated<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    terminator: u8,
    first: bool,
}

impl<'a, 'de> CommaSeparated<'a, 'de> {
    fn new(terminator: u8, de: &'a mut Deserializer<'de>) -> Self {
        CommaSeparated { de, terminator, first: true }
    }
}

impl<'de, 'a> de::SeqAccess<'de> for CommaSeparated<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
        where T: DeserializeSeed<'de>
    {
        // Check if there are no more elements.
        if self.de.input.current() == Some(self.terminator) {
            return Ok(None)
        }
        // Comma is required before every element except the first.
        if !self.first {
            if comma().parse(&mut self.de.input).is_err() {
                return Err(Error::ExpectedArrayComma);
            }
            if self.de.input.current() == Some(self.terminator) {
                return Ok(None)
            }
        }
        self.first = false;
        let _ = space().parse(&mut self.de.input);
        // Deserialize an array element.
        seed.deserialize(&mut *self.de).map(Some)
    }
}

impl<'de, 'a> de::MapAccess<'de> for CommaSeparated<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
        where K: DeserializeSeed<'de>
    {
        // Check if there are no more elements.
        if self.de.input.current() == Some(self.terminator) {
            return Ok(None)
        }
        // Comma is required before every element except the first.
        if !self.first {
            if comma().parse(&mut self.de.input).is_err() {
                return Err(Error::ExpectedMapComma);
            }
            if self.de.input.current() == Some(self.terminator) {
                return Ok(None)
            }
        }
        self.first = false;
        let _ = space().parse(&mut self.de.input);
        // Deserialize a map key.
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
        where V: DeserializeSeed<'de>
    {
        let parser = space() * sym(b':') - space();
        match parser.parse(&mut self.de.input) {
            Ok(_) => seed.deserialize(&mut *self.de),
            Err(_) => Err(Error::ExpectedMapColon),
        }
    }
}

struct Enum<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Enum { de: de }
    }
}

impl<'de, 'a> de::EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
        where V: DeserializeSeed<'de>
    {
        let value = seed.deserialize(&mut *self.de)?;
        Ok((value, self))
    }
}

impl<'de, 'a> de::VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
        where T: DeserializeSeed<'de>
    {
        match self.de.consume("(") {
            Ok(_) => {
                let value = seed.deserialize(&mut *self.de)?;
                let _ = comma().parse(&mut self.de.input);
                self.de.consume(")")
                    .map(|_| value)
                    .map_err(|_| Error::ExpectedStructEnd)
            },
            Err(_) => Err(Error::ExpectedStruct)
        }
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.de.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
        where V: Visitor<'de>
    {
        self.de.deserialize_struct("", fields, visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Deserialize)]
    struct EmptyStruct1;

    #[derive(Debug, PartialEq, Deserialize)]
    struct EmptyStruct2 {}

    #[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
    struct MyStruct { x: f32, y: f32 }

    #[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
    enum MyEnum {
        A,
        B(bool),
        C(bool, f32),
        D { a: i32, b: i32 }
    }

    #[test]
    fn test_empty_struct() {
        assert_eq!(Ok(EmptyStruct1), from_str("EmptyStruct1"));
        assert_eq!(Ok(EmptyStruct2{}), from_str("EmptyStruct2()"));
    }


    #[test]
    fn test_struct() {
        let my_struct = MyStruct { x: 4.0, y: 7.0 };

        assert_eq!(Ok(my_struct), from_str("MyStruct(x:4,y:7,)"));
        assert_eq!(Ok(my_struct), from_str("(x:4,y:7)"));

        #[derive(Debug, PartialEq, Deserialize)]
        struct NewType(i32);

        assert_eq!(Ok(NewType(42)), from_str("NewType(42)"));
        assert_eq!(Ok(NewType(33)), from_str("(33)"));

        #[derive(Debug, PartialEq, Deserialize)]
        struct TupleStruct(f32, f32);

        assert_eq!(Ok(TupleStruct(2.0, 5.0)), from_str("TupleStruct(2,5,)"));
        assert_eq!(Ok(TupleStruct(3.0, 4.0)), from_str("(3,4)"));
    }


    #[test]
    fn test_option() {
        assert_eq!(Ok(Some(1u8)), from_str("Some(1)"));
        assert_eq!(Ok(None::<u8>), from_str("None"));
    }

    #[test]
    fn test_enum() {
        assert_eq!(Ok(MyEnum::A), from_str("A"));
        assert_eq!(Ok(MyEnum::B(true)), from_str("B(true,)"));
        assert_eq!(Ok(MyEnum::C(true, 3.5)), from_str("C(true,3.5,)"));
        assert_eq!(Ok(MyEnum::D { a: 2, b: 3 }), from_str("D(a:2,b:3,)"));
    }

    #[test]
    fn test_array() {
        let empty: [i32; 0] = [];
        assert_eq!(Ok(empty), from_str("()"));
        let empty_array = empty.to_vec();
        assert_eq!(Ok(empty_array), from_str("[]"));

        assert_eq!(Ok([2, 3, 4i32]), from_str("(2,3,4,)"));
        assert_eq!(Ok(([2, 3, 4i32].to_vec())), from_str("[2,3,4,]"));
    }

    #[test]
    fn test_map() {
        use std::collections::HashMap;

        let mut map = HashMap::new();
        map.insert((true, false), 4);
        map.insert((false, false), 123);

        assert_eq!(Ok(map), from_str("{
            (true,false,):4,
            (false,false,):123,
        }"));
    }

    #[test]
    fn test_string() {
        let s: String = from_str("\"String\"").unwrap();

        assert_eq!("String", s);
    }

    #[test]
    fn test_char() {
        assert_eq!(Ok('c'), from_str("'c'"));
    }

    #[test]
    fn test_escape_char() {
        assert_eq!('\'', from_str::<char>("'\\''").unwrap());
    }

    #[test]
    fn test_escape() {
        assert_eq!("\"Quoted\"", from_str::<String>(r#""\"Quoted\"""#).unwrap());
    }
}
