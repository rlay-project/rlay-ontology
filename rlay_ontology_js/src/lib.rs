#![allow(non_snake_case)]

use rlay_ontology::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn getEntityCid(val: JsValue) -> JsValue {
    let web3_value: FormatWeb3<Entity> = val.into_serde().unwrap();
    let serde_value = serde_json::to_value(web3_value).unwrap();

    JsValue::from_serde(&serde_value["cid"]).unwrap()
}
