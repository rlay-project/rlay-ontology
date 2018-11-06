extern crate cid;
#[macro_use]
extern crate failure;
extern crate integer_encoding;
extern crate multibase;
extern crate multihash;
extern crate prost;
#[macro_use]
extern crate prost_derive;
extern crate rustc_hex;
extern crate serde;
#[macro_use]
extern crate serde_derive;

#[cfg(feature = "web3_compat")]
extern crate web3;

use std::io::Cursor;
use cid::{Cid, Codec, Error as CidError, ToCid, Version};
use integer_encoding::VarIntReader;
use std::collections::BTreeMap;

// Include the `items` module, which is generated from items.proto.
pub mod ontology {
    use multihash::encode;
    use multihash::Hash;
    use prost::Message;
    use cid::{Cid, Codec, Error as CidError, ToCid, Version};
    use rustc_hex::{FromHex, ToHex};

    use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};

    pub trait Canonicalize {
        fn canonicalize(&mut self);
    }

    pub trait AssociatedCodec {
        const CODEC_CODE: u64;
    }

    struct HexString<'a> {
        pub inner: &'a [u8],
    }

    impl<'a> HexString<'a> {
        pub fn wrap(bytes: &'a [u8]) -> Self {
            HexString { inner: bytes }
        }

        pub fn wrap_option(bytes: Option<&'a Vec<u8>>) -> Option<Self> {
            match bytes {
                Some(bytes) => Some(HexString { inner: bytes }),
                None => None,
            }
        }
    }

    impl<'a> ::serde::Serialize for HexString<'a> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ::serde::Serializer,
        {
            let hex: String = self.inner.to_hex();
            Ok(try!(serializer.serialize_str(&format!("0x{}", &hex))))
        }
    }

    include!(concat!(env!("OUT_DIR"), "/rlay.ontology.rs"));

    include!("./rlay.ontology.macros.rs");
    include!(concat!(env!("OUT_DIR"), "/rlay.ontology.macros_applied.rs"));

    // TODO: generate all of these from ontology intermediate.json
    impl_canonicalize!(Annotation; annotations);

    impl EntityKind {
        pub fn from_event_name(event_name: &str) -> Result<Self, ()> {
            let name = event_name.replace("Stored", "");

            Self::from_name(&name)
        }

        pub fn retrieve_fn_name(&self) -> String {
            format!("retrieve{}", Into::<&str>::into(self.to_owned()))
        }
    }

    impl Entity {
        pub fn to_bytes(&self) -> Vec<u8> {
            self.to_cid().unwrap().to_bytes()
        }

        pub fn get_subject(&self) -> Option<&Vec<u8>> {
            match &self {
                Entity::ClassAssertion(ent) => Some(ent.get_subject()),
                Entity::NegativeClassAssertion(ent) => Some(ent.get_subject()),
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
    pub use self::web3::*;

    #[cfg(feature = "web3_compat")]
    mod web3 {
        use super::*;

        use web3::types::U256;

        pub trait Web3Format<'a> {
            type Formatted: serde::Deserialize<'a> + serde::Serialize;

            fn to_web3_format(self) -> Self::Formatted;

            fn from_web3_format(formatted: Self::Formatted) -> Self;
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
            ($bytes_var:ident, $offset_var:ident, $start:expr, $end:expr) => (
                let $offset_var = U256::from_big_endian(&$bytes_var[$start..$end]);
            );
        }

        macro_rules! decode_param {
            (bytes_array; $bytes_var:ident, $param_var:ident, $start:expr, $end:expr) => (
                let $param_var = decode_bytes_array(
                    &$bytes_var[($start.as_u64() as usize)..($end.as_u64() as usize)],
                );
            );
            (bytes_array; $bytes_var:ident, $param_var:ident, $start:expr) => (
                let $param_var = decode_bytes_array(
                    &$bytes_var[($start.as_u64() as usize)..$bytes_var.len()],
                );
            );
            (bytes; $bytes_var:ident, $param_var:ident, $start:expr, $end:expr) => (
                let $param_var = decode_bytes(
                    &$bytes_var[($start.as_u64() as usize)..($end.as_u64() as usize)],
                );
            );
            (bytes; $bytes_var:ident, $param_var:ident, $start:expr) => (
                let $param_var = decode_bytes(
                    &$bytes_var[($start.as_u64() as usize)..$bytes_var.len()],
                );
            );
        }

        include!(concat!(env!("OUT_DIR"), "/rlay.ontology.web3_applied.rs"));

        impl<'a> Web3Format<'a> for Entity {
            type Formatted = EntityWeb3Format;

            fn to_web3_format(self) -> Self::Formatted {
                EntityWeb3Format::from(self)
            }

            fn from_web3_format(formatted: Self::Formatted) -> Self {
                formatted.into()
            }
        }
    }

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
            pub fn get_subject(&self) -> &Vec<u8> {
                &self.subject
            }
        }

        impl NegativeClassAssertion {
            pub fn get_subject(&self) -> &Vec<u8> {
                &self.subject
            }
        }
    }
}

pub const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";

#[derive(Fail, Debug)]
pub enum HashError {
    #[fail(display = "Multihash error: {}", error)] MultihashError { error: multihash::Error },
    #[fail(display = "Encoding error: {}", error)] EncodingError { error: prost::EncodeError },
}

impl From<multihash::Error> for HashError {
    fn from(error: multihash::Error) -> HashError {
        HashError::MultihashError { error }
    }
}

impl From<prost::EncodeError> for HashError {
    fn from(error: prost::EncodeError) -> HashError {
        HashError::EncodingError { error }
    }
}

// pub fn create_annotation(iri: String, value: String) -> Result<Annotation, multihash::Error> {
// let label_property = encode(Hash::SHA3256, &iri.as_bytes())?;
// Ok(Annotation::new(&label_property, value))
// }

// pub fn create_label_annotation(value: String) -> Result<Annotation, multihash::Error> {
// let label_property = encode(Hash::SHA3256, &RDFS_LABEL.as_bytes())?;
// Ok(Annotation::new(&label_property, value))
// }

pub trait ContentAddressableStorage<T> {
    type Error;

    fn insert_content(&mut self, val: T) -> Result<(), Self::Error>;
    fn get_content(&self, cid: &Cid) -> Option<&T>;
}

impl<T: ToCid> ContentAddressableStorage<T> for BTreeMap<String, T> {
    type Error = CidError;

    fn insert_content(&mut self, val: T) -> Result<(), CidError> {
        let cid_str = val.to_cid()?.to_string();
        self.insert(cid_str, val);
        Ok(())
    }

    fn get_content(&self, cid: &Cid) -> Option<&T> {
        let cid_str = cid.to_string();
        self.get(&cid_str)
    }
}

pub trait ToCidUnknown {
    fn to_cid_unknown(&self, permitted: Option<u64>) -> Result<Cid, CidError>;
}

impl ToCidUnknown for String {
    fn to_cid_unknown(&self, permitted: Option<u64>) -> Result<Cid, CidError> {
        let bytes = multibase::decode(self).unwrap().1;
        bytes.to_cid_unknown(permitted)
    }
}

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
