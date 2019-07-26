#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "pwasm", feature(alloc))]
// #![cfg_attr(all(feature = "wasm_bindgen", nightly), feature(custom_attribute))]

#[cfg(feature = "serde")]
extern crate serde;
#[cfg(feature = "serde_derive")]
#[macro_use]
extern crate serde_derive;

#[cfg(feature = "std")]
extern crate cid;
#[cfg(feature = "std")]
extern crate integer_encoding;
#[cfg(feature = "std")]
extern crate multibase;
#[cfg(feature = "std")]
extern crate multihash;
#[cfg(feature = "std")]
extern crate prost;
#[cfg(feature = "std")]
#[macro_use]
extern crate prost_derive;
#[cfg(feature = "std")]
extern crate rustc_hex;
#[cfg(feature = "std")]
extern crate serde_bytes;

#[cfg(feature = "pwasm")]
extern crate pwasm_std;

#[cfg(feature = "std")]
use cid::{Cid, Codec, Error as CidError, Version};
#[cfg(feature = "std")]
use integer_encoding::VarIntReader;

pub mod prelude {
    #[cfg(feature = "serde")]
    pub use crate::ontology::compact::*;
    #[cfg(feature = "std")]
    pub use crate::ontology::v0::*;
    #[cfg(feature = "web3_compat")]
    pub use crate::ontology::web3::*;
    pub use crate::ontology::*;
}

// Include the `items` module, which is generated from items.proto.
pub mod ontology {
    #[cfg(feature = "web3_compat")]
    use self::web3::{FromABIV2Response, FromABIV2ResponseHinted};
    #[cfg(feature = "std")]
    use cid::{Cid, Codec, Error as CidError, ToCid, Version};
    #[cfg(feature = "std")]
    use multihash::encode;
    #[cfg(feature = "std")]
    use multihash::Hash;
    #[cfg(feature = "std")]
    use prost::Message;
    #[cfg(feature = "pwasm")]
    use pwasm_std::*;
    #[cfg(feature = "std")]
    use serde::de::{Deserialize, Deserializer};
    #[cfg(feature = "wasm_bindgen")]
    use wasm_bindgen::prelude::*;

    pub trait Canonicalize {
        fn canonicalize(&mut self);
    }

    pub trait AssociatedCodec {
        const CODEC_CODE: u64;
    }

    pub trait CidFields<'a> {
        type Iter: Iterator<Item = &'a Vec<u8>>;

        fn iter_cid_fields(&'a self) -> Self::Iter;
    }

    // include!(concat!(env!("OUT_DIR"), "/rlay.ontology.rs"));
    include!(concat!(env!("OUT_DIR"), "/rlay.ontology.entities.rs"));

    include!("./rlay.ontology.macros.rs");
    include!(concat!(env!("OUT_DIR"), "/rlay.ontology.macros_applied.rs"));

    impl EntityKind {
        pub fn from_event_name(event_name: &str) -> Result<Self, ()> {
            let name = event_name.replace("Stored", "");

            Self::from_name(&name)
        }

        pub fn retrieve_fn_name(&self) -> String {
            format!("retrieve{}", Into::<&str>::into(self))
        }
    }

    impl Entity {
        #[cfg(feature = "std")]
        pub fn to_bytes(&self) -> Vec<u8> {
            self.to_cid().unwrap().to_bytes()
        }

        pub fn get_subject(&self) -> Option<&Vec<u8>> {
            match &self {
                Entity::ClassAssertion(ent) => ent.get_subject(),
                Entity::NegativeClassAssertion(ent) => ent.get_subject(),
                _ => None,
            }
        }

        pub fn as_class_assertion(&self) -> Option<&ClassAssertion> {
            match *self {
                Entity::ClassAssertion(ref val) => Some(&*val),
                _ => None,
            }
        }

        pub fn as_negative_class_assertion(&self) -> Option<&NegativeClassAssertion> {
            match *self {
                Entity::NegativeClassAssertion(ref val) => Some(&*val),
                _ => None,
            }
        }
    }

    pub use self::custom::*;

    #[cfg(feature = "web3_compat")]
    /// Serialization format compatible with the Web3 ecosystem, specifically the Web3 JSONRPC.
    pub mod web3 {
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
                let $param_var = decode_bytes_array(
                    &$bytes_var[($start.as_u64() as usize)..($end.as_u64() as usize)],
                );
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
                let $param_var =
                    decode_bytes(&$bytes_var[($start.as_u64() as usize)..$bytes_var.len()]);
            };
        }

        include!(concat!(env!("OUT_DIR"), "/rlay.ontology.web3_applied.rs"));

    }

    /// Compact serialization format that allows for omitting empty fields.
    #[cfg(feature = "serde")]
    pub mod compact {
        use super::*;

        pub trait FormatCompact<'a> {
            type Formatted: serde::Deserialize<'a> + serde::Serialize;

            fn to_compact_format(self) -> Self::Formatted;

            fn from_compact_format(formatted: Self::Formatted) -> Self;
        }

        include!(concat!(env!("OUT_DIR"), "/rlay.ontology.compact.rs"));
    }

    /// Hand-written extension traits that expose values common over some of the entity kinds.
    mod custom {
        use super::*;

        pub trait GetAssertionComplement {
            type Complement;

            fn get_assertion_complement(&self) -> Self::Complement;
        }

        impl GetAssertionComplement for ClassAssertion {
            type Complement = NegativeClassAssertion;

            fn get_assertion_complement(&self) -> Self::Complement {
                NegativeClassAssertion {
                    annotations: vec![],
                    subject: self.subject.clone(),
                    class: self.class.clone(),
                }
            }
        }

        impl GetAssertionComplement for NegativeClassAssertion {
            type Complement = ClassAssertion;

            fn get_assertion_complement(&self) -> Self::Complement {
                ClassAssertion {
                    annotations: vec![],
                    subject: self.subject.clone(),
                    class: self.class.clone(),
                }
            }
        }

        impl ClassAssertion {
            pub fn get_subject(&self) -> Option<&Vec<u8>> {
                self.subject.as_ref()
            }
        }

        impl NegativeClassAssertion {
            pub fn get_subject(&self) -> Option<&Vec<u8>> {
                self.subject.as_ref()
            }
        }
    }

    /// Serialization format for the canonical v0 cbor-based format.
    #[cfg(feature = "std")]
    pub mod v0 {
        use super::*;
        use crate::ontology::compact::FormatCompact;
        use integer_encoding::VarIntReader;
        use integer_encoding::VarIntWriter;

        include!(concat!(env!("OUT_DIR"), "/rlay.ontology.v0.rs"));
    }
}

#[cfg(feature = "std")]
pub trait ToCidUnknown {
    fn to_cid_unknown(&self, permitted: Option<u64>) -> Result<Cid, CidError>;
}

#[cfg(feature = "std")]
impl ToCidUnknown for String {
    fn to_cid_unknown(&self, permitted: Option<u64>) -> Result<Cid, CidError> {
        let bytes = multibase::decode(self).unwrap().1;
        bytes.to_cid_unknown(permitted)
    }
}

#[cfg(feature = "std")]
use std::io::Cursor;
#[cfg(feature = "std")]
impl ToCidUnknown for [u8] {
    fn to_cid_unknown(&self, permitted: Option<u64>) -> Result<Cid, CidError> {
        let mut cur = Cursor::new(self);
        let raw_version = cur.read_varint()?;
        let raw_codec = cur.read_varint()?;

        let version = Version::from(raw_version)?;
        match permitted {
            Some(permitted) => {
                if raw_codec != permitted {
                    return Err(CidError::UnknownCodec);
                }
            }
            None => {}
        }
        let codec = Codec::Unknown(raw_codec);
        let hash = &self[cur.position() as usize..];

        multihash::decode(hash)?;

        Ok(Cid::new(codec, version, hash))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
