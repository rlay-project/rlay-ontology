#![allow(non_snake_case)]
use heck::SnakeCase;
use std::collections::BTreeMap;
use syn;

pub fn parse_intermediate_contents(contents: &str) -> Intermediate {
    serde_json::from_str(contents).unwrap()
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Intermediate {
    pub kinds: Vec<Kind>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Kind {
    pub name: String,
    pub fields: Vec<Field>,
    pub expressionKind: Option<String>,
    pub kindId: u64,
    pub cidPrefix: u64,
    pub cidPrefixHex: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Field {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub required: bool,
}

impl Field {
    pub fn is_array_kind(&self) -> bool {
        self.kind.ends_with("[]")
    }

    pub fn field_ident(&self) -> syn::Ident {
        syn::parse_str(&self.name.to_snake_case()).unwrap()
    }
}
