extern crate cid;
extern crate itertools;
extern crate multibase;
extern crate multihash;
extern crate spread_ontology;

use multihash::encode;
use multihash::Hash;
use itertools::Itertools;
use multibase::{encode as base_encode, Base};
use spread_ontology::ontology::{Annotation, Class};
use cid::ToCid;
use std::collections::BTreeMap;

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
            "{:?}",
            self.0
                .chunks(1)
                .map(|n| n.iter().map(|m| format!("0x{:02x}", m)).format(""))
                .collect::<Vec<_>>()
        );
        Ok(())
    }
}

pub fn main() {
    println!(
        "{}",
        SolidityBytesChunked(
            &spread_ontology::create_label_annotation(String::new())
                .unwrap()
                .property
        )
    );
    println!(
        "\"label\" annotation property: {}",
        SolidityBytes(
            &spread_ontology::create_label_annotation(String::new())
                .unwrap()
                .property
        ),
    );
    let label_annotation = spread_ontology::create_label_annotation("Organization".to_owned())
        .unwrap()
        .to_cid()
        .unwrap();
    let label_hash = label_annotation.to_bytes();
    println!("Byte part cid: {}", SolidityBytes(&label_annotation.hash));
    println!("Full cid: {}", SolidityBytes(&label_hash));
    println!("Full cid: {}", SolidityBytesChunked(&label_hash));
    let base58_label_hash = base_encode(Base::Base58btc, &label_hash);
    println!("Full cid (base58btc): {}", base58_label_hash);

    let mut organization = Class::default();
    organization.annotations.push(label_hash);
    let organization_cid = organization.to_cid().unwrap();
    let organization_hash = organization_cid.to_bytes();
    println!("======= Class Organization ======");
    println!("Byte part cid: {}", SolidityBytes(&organization_cid.hash));
    println!("Full cid: {}", SolidityBytes(&organization_hash));

    let mut company = Class::default();
    company.sub_class_of_class.push(organization_hash);
    let company_cid = company.to_cid().unwrap();
    let company_hash = company_cid.to_bytes();
    println!("======= Class Company ======");
    println!("Byte part cid: {}", SolidityBytes(&company_cid.hash));
    println!("Full cid: {}", SolidityBytes(&company_hash));
}
