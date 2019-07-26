//! Serialization format compatible with the Web3 ecosystem, specifically the Web3 JSONRPC.
use super::*;
use ethereum_types::U256;
use rustc_hex::{FromHex, ToHex};
use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};
use serde::ser::{Serialize, SerializeSeq, SerializeStruct};

#[derive(Clone)]
pub struct FormatWeb3<T: Clone>(pub T);

pub trait SerializeFormatWeb3 {
    fn serialize_format_web3<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer;
}

impl<T: SerializeFormatWeb3 + Clone> serde::Serialize for FormatWeb3<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        SerializeFormatWeb3::serialize_format_web3(&self.0, serializer)
    }
}

impl<T: SerializeFormatWeb3 + Clone> From<T> for FormatWeb3<T> {
    fn from(original: T) -> Self {
        FormatWeb3(original)
    }
}

impl<T: SerializeFormatWeb3 + Clone> SerializeFormatWeb3 for &T {
    fn serialize_format_web3<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        SerializeFormatWeb3::serialize_format_web3(*self, serializer)
    }
}

impl<T: SerializeFormatWeb3 + Clone> SerializeFormatWeb3 for Vec<T> {
    fn serialize_format_web3<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for entry in self.iter() {
            seq.serialize_element(&FormatWeb3(entry))?;
        }
        seq.end()
    }
}

impl<T: SerializeFormatWeb3 + Clone> SerializeFormatWeb3 for Option<T> {
    fn serialize_format_web3<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Some(inner) => SerializeFormatWeb3::serialize_format_web3(inner, serializer),
            None => serializer.serialize_none(),
        }
    }
}

impl SerializeFormatWeb3 for Vec<u8> {
    fn serialize_format_web3<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("0x{}", self.to_hex::<String>()))
    }
}

pub trait DeserializeFormatWeb3<'de>: Sized {
    fn deserialize_format_web3<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>;
}

impl<'de, T: DeserializeFormatWeb3<'de> + Clone> Deserialize<'de> for FormatWeb3<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        DeserializeFormatWeb3::deserialize_format_web3(deserializer)
    }
}

impl<'de, T: DeserializeFormatWeb3<'de> + Clone> DeserializeFormatWeb3<'de> for FormatWeb3<T> {
    fn deserialize_format_web3<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(FormatWeb3(DeserializeFormatWeb3::deserialize_format_web3(
            deserializer,
        )?))
    }
}

impl<'de> DeserializeFormatWeb3<'de> for Vec<u8> {
    fn deserialize_format_web3<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringVisitor;

        impl<'de> Visitor<'de> for StringVisitor {
            type Value = Vec<u8>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a hex encoded string prefixed by 0x")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if &s[0..2] != "0x" {
                    return Err(de::Error::invalid_value(de::Unexpected::Str(s), &self));
                }
                Ok(s[2..].from_hex().map_err(de::Error::custom)?)
            }
        }

        deserializer.deserialize_str(StringVisitor)
    }
}

/// Decode a single ethabi param of type bytes
fn decode_bytes(bytes: &[u8]) -> Vec<u8> {
    let length = U256::from_big_endian(&bytes[0..32]);
    bytes[((32) as usize)..((length).as_u64() as usize + 32)].to_owned()
}

/// Decode a single ethabi param of type bytes[]
fn decode_bytes_array(bytes: &[u8]) -> Vec<Vec<u8>> {
    let num_elements = U256::from_big_endian(&bytes[0..32]);

    let element_offsets: Vec<U256> = (0..num_elements.as_u64())
        .map(|element_i| {
            let element_data_offset = U256::from_big_endian(
                // additional offset of 1 to account for leading word that holds the number of elements
                &bytes[(32 * (element_i + 1) as usize)..(32 * (element_i + 2) as usize)],
            );
            // + 32 because of leading word
            element_data_offset + Into::<U256>::into(32)
        })
        .collect();

    element_offsets
        .into_iter()
        .map(|element_start_offset| {
            decode_bytes(&bytes[(element_start_offset.as_u64() as usize)..bytes.len()])
        })
        .collect()
}

fn to_option_bytes(bytes: Vec<u8>) -> Option<Vec<u8>> {
    match bytes.len() {
        0 => None,
        _ => Some(bytes),
    }
}

pub trait FromABIV2Response {
    fn from_abiv2(bytes: &[u8]) -> Self;
}

pub trait FromABIV2ResponseHinted {
    fn from_abiv2(bytes: &[u8], kind: &EntityKind) -> Self;
}

macro_rules! decode_offset {
    ($bytes_var:ident, $offset_var:ident, $start:expr, $end:expr) => {
        let $offset_var = U256::from_big_endian(&$bytes_var[$start..$end]);
    };
}

macro_rules! decode_param {
    (bytes_array; $bytes_var:ident, $param_var:ident, $start:expr, $end:expr) => {
        let $param_var =
            decode_bytes_array(&$bytes_var[($start.as_u64() as usize)..($end.as_u64() as usize)]);
    };
    (bytes_array; $bytes_var:ident, $param_var:ident, $start:expr) => {
        let $param_var =
            decode_bytes_array(&$bytes_var[($start.as_u64() as usize)..$bytes_var.len()]);
    };
    (bytes; $bytes_var:ident, $param_var:ident, $start:expr, $end:expr) => {
        let $param_var =
            decode_bytes(&$bytes_var[($start.as_u64() as usize)..($end.as_u64() as usize)]);
    };
    (bytes; $bytes_var:ident, $param_var:ident, $start:expr) => {
        let $param_var = decode_bytes(&$bytes_var[($start.as_u64() as usize)..$bytes_var.len()]);
    };
}

include!(concat!(env!("OUT_DIR"), "/rlay.ontology.web3_applied.rs"));
