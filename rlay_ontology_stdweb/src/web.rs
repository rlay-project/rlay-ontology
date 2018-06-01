#![recursion_limit = "256"]

extern crate cid;
extern crate hobofan_stdweb_logger as stdweb_logger;
extern crate itertools;
#[macro_use]
extern crate log;
extern crate multibase;
extern crate multihash;
extern crate petgraph;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate stdweb;

extern crate rlay_ontology;

mod serializable_types;

use cid::ToCid;
use itertools::Itertools;
use multibase::{decode as mb_decode, encode as mb_encode, Base};
use multihash::{decode as mh_decode, encode as mh_encode, Hash};
use petgraph::dot::{Config, Dot};
use petgraph::graph::NodeIndex;
use petgraph::visit::IntoNodeReferences;
use petgraph::{Graph, Undirected};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt::Write;
use rlay_ontology::ontology::{Annotation, Canonicalize, Class, Individual as OntologyIndividual};

use serializable_types::*;

fn print_class(class: SerializableClass) {
    debug!("{:?}", &class);
}

fn annotation_property_label() -> String {
    let property_hash = mh_encode(Hash::SHA3256, &rlay_ontology::RDFS_LABEL.as_bytes()).unwrap();
    mb_encode(Base::Base58btc, property_hash)
}

fn hash_annotation(val: SerializableAnnotation) -> String {
    let annotation: Annotation = val.into();
    let annotation_cid = annotation.to_cid().unwrap();
    annotation_cid.to_string()
}

fn hash_class(val: SerializableClassV2) -> String {
    let mut class: Class = val.into();
    class.canonicalize();
    let class_cid = class.to_cid().unwrap();
    class_cid.to_string()
}

fn hash_individual(val: SerializableIndividualV2) -> String {
    let mut item: OntologyIndividual = val.into();
    item.canonicalize();
    let item_cid = item.to_cid().unwrap();
    item_cid.to_string()
}

fn main() {
    stdweb::initialize();
    // stdweb_logger::Logger::init_with_level(::log::LevelFilter::Trace);
    stdweb_logger::Logger::init_with_level(::log::LevelFilter::Debug);

    js! {
        Module.exports.print_class = @{print_class};
        Module.exports.annotation_property_label = @{annotation_property_label};
        Module.exports.hash_annotation = @{hash_annotation};
        Module.exports.hash_class = @{hash_class};
        Module.exports.hash_individual = @{hash_individual};
    }
}
