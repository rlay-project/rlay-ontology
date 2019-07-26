#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "pwasm", feature(alloc))]
// #![cfg_attr(all(feature = "wasm_bindgen", nightly), feature(custom_attribute))]

#[cfg(feature = "serde")]
extern crate serde;
#[cfg(feature = "serde_derive")]
#[macro_use]
extern crate serde_derive;

#[cfg(feature = "std")]
use cid::{Cid, Codec, Error as CidError, Version};
#[cfg(feature = "std")]
use integer_encoding::VarIntReader;

pub mod ontology;
pub mod prelude {
    #[cfg(feature = "serde")]
    pub use crate::ontology::compact::*;
    #[cfg(feature = "std")]
    pub use crate::ontology::v0::*;
    #[cfg(feature = "web3_compat")]
    pub use crate::ontology::web3::*;
    pub use crate::ontology::*;
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
