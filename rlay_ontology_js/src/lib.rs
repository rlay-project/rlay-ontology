#![allow(non_snake_case)]

use cid_fork_rlay::ToCid;
use rlay_ontology::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn getEntityCid(val: JsValue) -> JsValue {
    let web3_value: FormatWeb3<Entity> = val.into_serde().unwrap();
    let cid_value = web3_value.0.to_cid().ok().map(|n| FormatWeb3(n.to_bytes()));

    JsValue::from_serde(&cid_value).unwrap()
}
