use rlay_ontology::prelude::*;

#[test]
fn entity_variants() {
    let entity_variants = EntityKind::variants();

    assert!(entity_variants.contains(&"Annotation"));
}

#[test]
fn annotation_data_field_names() {
    assert!(Annotation::data_field_names().contains(&"value"));
}

#[test]
fn annotation_cid_field_names() {
    assert!(Annotation::cid_field_names().contains(&"property"));
}
