#[macro_use]
extern crate serde_json;

use rlay_ontology::prelude::*;

#[test]
fn ignores_cid_field() {
    let content = json!({
        "cid": "0x1234",
        "type": "Annotation",
        "property": "0x",
        "value": "0x"
    });

    let parsed_annotation: FormatWeb3<Entity> = serde_json::from_value(content).unwrap();
    let expected_annotation: Entity = Annotation::default().into();

    assert_eq!(expected_annotation, parsed_annotation.0);
}
