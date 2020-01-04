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

#[test]
fn call_with_entity_kinds() {
    let mut _abc = vec![];

    macro_rules! test_field_names {
        ($kind:path) => {
            _abc = <$kind>::data_field_names().into_iter().collect();
        };
    }

    rlay_ontology::call_with_entity_kinds!(ALL; test_field_names!);
}
