use rlay_ontology::prelude::*;

#[test]
fn entity_variants() {
    let entity_variants = EntityKind::variants();

    assert!(entity_variants.contains(&"Annotation"));
}
