use cid::ToCid;
use prost::Message;
use rlay_ontology::ontology::*;
use rustc_hex::FromHex;

#[test]
fn protobuf_format_encoding() {
    let klass = Class {
        annotations: vec![b"\x01\x02\x03".to_vec()],
        ..Class::default()
    };

    let mut encoded_klass = Vec::<u8>::new();
    klass.encode(&mut encoded_klass).unwrap();

    let expected_bytes = b"\x0a\x03\x01\x02\x03".to_vec();
    assert_eq!(expected_bytes, encoded_klass);
}

#[test]
fn protobuf_format_decoding() {
    let bytes = b"\x0a\x05\x01\x02\x03\x02\x03".to_vec();

    let expected_klass = Class {
        annotations: vec![b"\x01\x02\x03\x02\x03".to_vec()],
        ..Class::default()
    };

    let decoded_klass = Class::decode(&bytes).unwrap();

    assert_eq!(expected_klass, decoded_klass);
}

#[test]
/// This highlights one of the shortcomings of the protobuf format: different entities are encoded
/// to the same bytes
fn protobuf_format_encoding_equal() {
    let ann = DataPropertyAssertion {
        annotations: vec![b"\x01\x02\x03".to_vec()],
        ..DataPropertyAssertion::default()
    };

    let klass = Class {
        annotations: vec![b"\x01\x02\x03".to_vec()],
        ..Class::default()
    };

    let mut encoded_ann = Vec::<u8>::new();
    ann.encode(&mut encoded_ann).unwrap();

    let mut encoded_klass = Vec::<u8>::new();
    klass.encode(&mut encoded_klass).unwrap();

    assert_eq!(encoded_ann, encoded_klass);
}

#[test]
fn protobuf_cid() {
    let klass = Class {
        annotations: vec![b"\x01\x02\x03".to_vec()],
        ..Class::default()
    };

    let cid = klass.to_cid().unwrap().to_bytes();

    let expected_bytes: Vec<u8> =
        "018080031b20e74c92dfbce4b3219c3106b978aa99b8c0b1a34f90e60c947615752d37c210f9"
            .from_hex()
            .unwrap();
    assert_eq!(expected_bytes, cid);
}
