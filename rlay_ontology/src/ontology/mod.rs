#[cfg(feature = "web3_compat")]
pub mod web3;

#[cfg(feature = "web3_compat")]
use self::web3::{FromABIV2Response, FromABIV2ResponseHinted};
#[cfg(feature = "std")]
use ambassador::delegatable_trait_remote;
use ambassador::{delegatable_trait, Delegate};
#[cfg(feature = "std")]
use cid_fork_rlay::{Cid, Codec, Error as CidError, ToCid, Version};
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

#[cfg_attr(feature = "std", delegatable_trait_remote)]
#[cfg(feature = "std")]
trait ToCid {
    fn to_cid(&self) -> Result<Cid, CidError>;
}

#[delegatable_trait]
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
