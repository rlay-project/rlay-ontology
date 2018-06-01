extern crate cid;
#[macro_use]
extern crate failure;
extern crate integer_encoding;
extern crate multibase;
extern crate multihash;
extern crate prost;
#[macro_use]
extern crate prost_derive;

use std::io::Cursor;
use multihash::encode;
use multihash::Hash;
use cid::{Cid, Codec, Error as CidError, ToCid, Version};
use integer_encoding::VarIntReader;
use std::collections::BTreeMap;

// Include the `items` module, which is generated from items.proto.
pub mod ontology {
    include!(concat!(env!("OUT_DIR"), "/spread.ontology.rs"));

    use multihash::encode;
    use multihash::Hash;
    use prost::Message;
    use cid::{Cid, Codec, Error as CidError, ToCid, Version};

    pub trait Canonicalize {
        fn canonicalize(&mut self);
    }

    pub trait AssociatedCodec {
        const CODEC_CODE: u64;
    }

    impl Annotation {
        pub fn new(property: &[u8], value: String) -> Self {
            Annotation {
                property: property.to_owned(),
                value,
            }
        }
    }

    impl AssociatedCodec for Annotation {
        const CODEC_CODE: u64 = 0xf0; // TODO: placeholder value
    }

    impl Canonicalize for Class {
        fn canonicalize(&mut self) {
            self.annotations.sort();
            self.sub_class_of_class.sort();
        }
    }

    impl AssociatedCodec for Class {
        const CODEC_CODE: u64 = 0xf1; // TODO: placeholder value
    }

    impl Canonicalize for Individual {
        fn canonicalize(&mut self) {
            self.annotations.sort();
            self.class_assertions.sort();
            self.negative_class_assertions.sort();
        }
    }

    impl AssociatedCodec for Individual {
        const CODEC_CODE: u64 = 0xf2; // TODO: placeholder value
    }

    macro_rules! toCidImpl {
        ($v:path) => (
            impl ToCid for $v {
                fn to_cid(&self) -> Result<Cid, CidError> {
                    let mut encoded = Vec::<u8>::new();
                    self.encode(&mut encoded).map_err(|_| CidError::ParsingError)?;
                    let hashed = encode(Hash::Keccak256, &encoded).map_err(|_| CidError::ParsingError)?;

                    let cid = Cid::new(Codec::Unknown(<Self as AssociatedCodec>::CODEC_CODE), Version::V1, &hashed);
                    Ok(cid)
                }
            })
        ;
    }

    toCidImpl!(Annotation);
    toCidImpl!(Class);
    toCidImpl!(Individual);
}

use ontology::Annotation;

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

pub fn create_annotation(iri: String, value: String) -> Result<Annotation, multihash::Error> {
    let label_property = encode(Hash::SHA3256, &iri.as_bytes())?;
    Ok(Annotation::new(&label_property, value))
}

pub fn create_label_annotation(value: String) -> Result<Annotation, multihash::Error> {
    let label_property = encode(Hash::SHA3256, &RDFS_LABEL.as_bytes())?;
    Ok(Annotation::new(&label_property, value))
}

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
    fn to_cid_unknown(&self, permitted: u64) -> Result<Cid, CidError>;
}

impl ToCidUnknown for String {
    fn to_cid_unknown(&self, permitted: u64) -> Result<Cid, CidError> {
        let bytes = multibase::decode(self).unwrap().1;
        bytes.to_cid_unknown(permitted)
    }
}

impl ToCidUnknown for [u8] {
    fn to_cid_unknown(&self, permitted: u64) -> Result<Cid, CidError> {
        let mut cur = Cursor::new(self);
        let raw_version = cur.read_varint()?;
        let raw_codec = cur.read_varint()?;

        let version = Version::from(raw_version)?;
        if raw_codec != permitted {
            // TODO: spelling; https://github.com/ipld/rust-cid/pull/10
            return Err(CidError::UnkownCodec);
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
