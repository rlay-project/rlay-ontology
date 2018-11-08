extern crate cid;
extern crate integer_encoding;
extern crate itertools;
extern crate multibase;
extern crate multihash;
extern crate prost;
extern crate rlay_ontology;
extern crate rustc_hex;
extern crate serde_cbor;
extern crate serde_json;

use itertools::Itertools;
use multibase::{encode as base_encode, Base};
use rlay_ontology::prelude::*;
use rustc_hex::FromHex;
use rustc_hex::ToHex;
use cid::ToCid;
use std::collections::BTreeMap;
use prost::Message;
use integer_encoding::VarInt;

pub struct AnnotationMap(BTreeMap<Vec<u8>, Annotation>);
pub struct ClassMap(BTreeMap<Vec<u8>, Class>);

struct SolidityBytes<'a>(&'a [u8]);

impl<'a> std::fmt::Display for SolidityBytes<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "0x{:02x}", self.0.iter().format(""))
    }
}

struct SolidityBytesChunked<'a>(&'a [u8]);

impl<'a> std::fmt::Display for SolidityBytesChunked<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .chunks(1)
                .map(|n| n.iter()
                    .map(|m| format!("{:02x}", m))
                    .collect::<Vec<_>>()
                    .join(""))
                .collect::<Vec<String>>()
                .join("")
        ).unwrap();
        Ok(())
    }
}

pub fn main() {
    // let mut annotation = Annotation::default();
    // annotation.value =
    // "019580031b2088868a58d3aac6d2558a29b3b8cacf3c9788364f57a3470158283121a15dcae0"
    // .from_hex()
    // .unwrap();
    // let serialized = serde_cbor::ser::to_vec_packed(&annotation).unwrap();

    // let mut serialized_pb = Vec::new();
    // annotation.encode(&mut serialized_pb).unwrap();

    // println!("Annotation CBOR: {}", SolidityBytesChunked(&serialized));
    // println!("Annotation PB  : {}", SolidityBytesChunked(&serialized_pb));

    let raw_encoded: Vec<u8> = "0480".from_hex().unwrap();
    println!("{:?}", raw_encoded);
    let decoded: u32 = VarInt::decode_var(&raw_encoded).0;
    println!("Decoded: {}", decoded);
    println!("Decoded Hex: {:x?}", decoded);

    let raw_value = vec![0xa2, 0x01, 0x41, 0xab, 0x02, 0x41, 0x45];
    let value: AnnotationFormatCompact = serde_cbor::from_slice(&raw_value).unwrap();
    let value = Annotation::from_compact_format(value).to_web3_format();
    println!("{}", serde_json::to_string_pretty(&value).unwrap());

    let value: AnnotationFormatCompact = serde_cbor::from_slice(&raw_value).unwrap();
    let value = Annotation::from_compact_format(value);
    let entity_v0 = EntityV0::Annotation(value);
    let mut entity_serialized: Vec<u8> = Vec::new();
    entity_v0.serialize(&mut entity_serialized);
    println!("{}", SolidityBytes(&entity_serialized));

    // println!(
    // "{}",
    // SolidityBytesChunked(
    // &rlay_ontology::create_label_annotation(String::new())
    // .unwrap()
    // .property
    // )
    // );
    // println!(
    // "\"label\" annotation property: {}",
    // SolidityBytes(
    // &rlay_ontology::create_label_annotation(String::new())
    // .unwrap()
    // .property
    // ),
    // );
    // let label_annotation = rlay_ontology::create_label_annotation("Organization".to_owned())
    // .unwrap()
    // .to_cid()
    // .unwrap();
    // let label_hash = label_annotation.to_bytes();
    // println!("Byte part cid: {}", SolidityBytes(&label_annotation.hash));
    // println!("Full cid: {}", SolidityBytes(&label_hash));
    // println!("Full cid: {}", SolidityBytesChunked(&label_hash));
    // let base58_label_hash = base_encode(Base::Base58btc, &label_hash);
    // println!("Full cid (base58btc): {}", base58_label_hash);

    // let mut organization = Class::default();
    // organization.annotations.push(label_hash);
    // let organization_cid = organization.to_cid().unwrap();
    // let organization_hash = organization_cid.to_bytes();
    // println!("======= Class Organization ======");
    // println!("Byte part cid: {}", SolidityBytes(&organization_cid.hash));
    // println!("Full cid: {}", SolidityBytes(&organization_hash));

    // let mut company = Class::default();
    // company.sub_class_of_class.push(organization_hash);
    // let company_cid = company.to_cid().unwrap();
    // let company_hash = company_cid.to_bytes();
    // println!("======= Class Company ======");
    // println!("Byte part cid: {}", SolidityBytes(&company_cid.hash));
    // println!("Full cid: {}", SolidityBytes(&company_hash));
}
