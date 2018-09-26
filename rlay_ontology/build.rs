#![allow(unused_imports)]
extern crate heck;
extern crate prost_build;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use std::env;
use std::fs::File;
use std::io::Write;
use std::io::prelude::*;
use std::path::Path;
use heck::SnakeCase;

use serde_json::Value;

fn main() {
    prost_build::compile_protos(&["src/ontology.proto"], &["src/"]).unwrap();
    main::build_macros_applied_file("src/intermediate.json", "rlay.ontology.macros_applied.rs");
    web3::build_applied_file("src/intermediate.json", "rlay.ontology.web3_applied.rs");
}

#[derive(Deserialize)]
struct Field {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub required: bool,
}

impl Field {
    pub fn is_array_kind(&self) -> bool {
        self.kind.ends_with("[]")
    }
}

mod main {
    use super::*;

    pub fn build_macros_applied_file(src_path: &str, out_path: &str) {
        let out_dir = env::var("OUT_DIR").unwrap();
        let dest_path = Path::new(&out_dir).join(out_path);

        let mut intermediate_file = File::open(src_path).expect("file not found");

        let mut intermediate_contents = String::new();
        intermediate_file
            .read_to_string(&mut intermediate_contents)
            .unwrap();
        let intermediate: Value = serde_json::from_str(&intermediate_contents).unwrap();

        let mut out_file = File::create(&dest_path).unwrap();

        let kinds = intermediate.as_object().unwrap()["kinds"]
            .as_array()
            .unwrap();
        for raw_kind in kinds {
            let kind = raw_kind.as_object().unwrap();

            let kind_name = kind["name"].as_str().unwrap();
            let kind_cid_prefix = kind["cidPrefix"].as_u64().unwrap();

            let fields: Vec<Field> = serde_json::from_value(kind["fields"].clone()).unwrap();

            // Header line
            write!(out_file, "\n// {}\n", kind_name).unwrap();
            // impl AssociatedCodec
            write!(
                out_file,
                "codec_code!({}, {});\n",
                kind_name, kind_cid_prefix
            ).unwrap();
            // impl ToCid
            write!(out_file, "impl_to_cid!({});\n", kind_name).unwrap();
            // impl ::serde::Serialize
            write_impl_serialize(&mut out_file, kind_name, &fields);

            write!(
                out_file,
                "impl_into_entity_kind!({0}, Entity::{0});\n",
                kind_name
            ).unwrap();
        }

        let kind_names: Vec<String> = kinds
            .iter()
            .map(|raw_kind| {
                let kind = raw_kind.as_object().unwrap();

                kind["name"].as_str().unwrap().to_owned()
            })
            .collect();
        write_entity_kind(&mut out_file, kind_names.clone());
        write_entity(&mut out_file, kind_names.clone());
    }

    fn write_impl_serialize(out_file: &mut File, kind_name: &str, fields: &[Field]) {
        write!(
            out_file,
            "
                impl ::serde::Serialize for {0} {{
                    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                    where
                        S: ::serde::Serializer,
                    {{
                        #[derive(Serialize)]
                        #[allow(non_snake_case)]
                        struct SerializeHelper<'a> {{
                            pub cid: Option<HexString<'a>>,
            ",
            kind_name
        ).unwrap();
        for field in fields.iter() {
            if field.is_array_kind() {
                write!(out_file, "pub {0}: Vec<HexString<'a>>,\n", field.name).unwrap();
            } else {
                if field.required {
                    write!(out_file, "pub {0}: HexString<'a>,\n", field.name).unwrap();
                } else {
                    write!(out_file, "pub {0}: Option<HexString<'a>>,\n", field.name).unwrap();
                }
            }
        }
        write!(
            out_file,
            "
                }}

                let cid_option = self.to_cid().ok().map(|n| n.to_bytes());
                let ext = SerializeHelper {{
                    cid: HexString::wrap_option(cid_option.as_ref()),
            "
        ).unwrap();
        for field in fields.iter() {
            if field.is_array_kind() {
                write!(
                    out_file,
                    "{0}: self.{1}.iter().map(|n| HexString::wrap(n)).collect(),\n",
                    field.name,
                    field.name.to_snake_case(),
                ).unwrap();
            } else {
                if field.required {
                    write!(
                        out_file,
                        "{0}: HexString::wrap(&self.{1}),\n",
                        field.name,
                        field.name.to_snake_case(),
                    ).unwrap();
                } else {
                    write!(
                        out_file,
                        "{0}: HexString::wrap_option(self.{1}.as_ref()),\n",
                        field.name,
                        field.name.to_snake_case(),
                    ).unwrap();
                }
            }
        }
        write!(
            out_file,
            "
                        }};

                        Ok(try!(ext.serialize(serializer)))
                    }}
                }}
            "
        ).unwrap();
    }

    fn write_entity_kind(out_file: &mut File, kind_names: Vec<String>) {
        write!(
            out_file,
            "
            #[derive(Debug, Clone, PartialEq, Serialize)]
            pub enum EntityKind {{
        "
        ).unwrap();
        for name in kind_names.iter() {
            write!(out_file, "{},\n", name).unwrap();
        }
        write!(
            out_file,
            "
            }}

            impl<'a> Into<&'a str> for EntityKind {{
                fn into(self) -> &'a str {{
                    match &self {{
        "
        ).unwrap();
        for name in kind_names.iter() {
            write!(out_file, "EntityKind::{0} => \"{0}\",\n", name).unwrap();
        }
        write!(
            out_file,
            "
                    }}
                }}
            }}

            impl EntityKind {{
                pub fn from_name(name: &str) -> Result<Self, ()> {{
                    match name {{
        "
        ).unwrap();
        for name in kind_names.iter() {
            write!(out_file, "\"{0}\" => Ok(EntityKind::{0}),\n", name).unwrap();
        }
        write!(
            out_file,
            "
                        _ => Err(()),
                    }}
                }}
            }}
        "
        ).unwrap();
    }

    fn write_entity(out_file: &mut File, kind_names: Vec<String>) {
        write!(
            out_file,
            "
            #[derive(Debug, Clone, PartialEq, Serialize)]
            #[serde(tag = \"type\")]
            pub enum Entity {{
        "
        ).unwrap();
        for name in kind_names.iter() {
            write!(out_file, "{0}({0}),\n", name).unwrap();
        }
        write!(
            out_file,
            "
            }}

            impl ToCid for Entity {{
                fn to_cid(&self) -> Result<Cid, CidError> {{
                    match &self {{
        "
        ).unwrap();
        for name in kind_names.iter() {
            write!(out_file, "Entity::{0}(ent) => ent.to_cid(),\n", name).unwrap();
        }
        write!(
            out_file,
            "
                    }}
                }}
            }}
        "
        ).unwrap();

        write!(
            out_file,
            "
            #[cfg(feature = \"web3_compat\")]
            impl FromABIV2ResponseHinted for Entity {{
                fn from_abiv2(bytes: &[u8], kind: &EntityKind) -> Self {{
                    match kind {{
        "
        ).unwrap();
        for name in kind_names.iter() {
            write!(
                out_file,
                "
                    EntityKind::{0} => {{
                        Entity::{0}(FromABIV2Response::from_abiv2(bytes))
                    }}
                ",
                name
            ).unwrap();
        }
        write!(
            out_file,
            "
                    }}
                }}
            }}
        "
        ).unwrap();

        write!(
            out_file,
            "
            impl Entity {{
                pub fn kind(&self) -> EntityKind {{
                    match &self {{
        "
        ).unwrap();
        for name in kind_names.iter() {
            write!(
                out_file,
                "
                    Entity::{0}(_) => EntityKind::{0},
                ",
                name
            ).unwrap();
        }
        write!(
            out_file,
            "
                    }}
                }}
            }}
        "
        ).unwrap();
    }
}

mod web3 {
    use super::*;

    pub fn build_applied_file(src_path: &str, out_path: &str) {
        let out_dir = env::var("OUT_DIR").unwrap();
        let dest_path = Path::new(&out_dir).join(out_path);

        let mut intermediate_file = File::open(src_path).expect("file not found");

        let mut intermediate_contents = String::new();
        intermediate_file
            .read_to_string(&mut intermediate_contents)
            .unwrap();
        let intermediate: Value = serde_json::from_str(&intermediate_contents).unwrap();

        let mut out_file = File::create(&dest_path).unwrap();

        let kinds = intermediate.as_object().unwrap()["kinds"]
            .as_array()
            .unwrap();

        for raw_kind in kinds {
            let kind = raw_kind.as_object().unwrap();

            let kind_name = kind["name"].as_str().unwrap();

            let fields: Vec<Field> = serde_json::from_value(kind["fields"].clone()).unwrap();

            // impl FromABIV2Response
            write!(
                out_file,
                "
                    impl FromABIV2Response for {0} {{
                        fn from_abiv2(bytes: &[u8]) -> Self {{
                ",
                kind_name
            ).unwrap();

            for (i, field) in fields.iter().enumerate() {
                write!(
                    out_file,
                    "decode_offset!(bytes, {0}_offset, {1}, {2});\n",
                    field.name.to_snake_case(),
                    i * 32,
                    (i + 1) * 32,
                ).unwrap();
            }
            for (i, field) in fields.iter().enumerate() {
                let next_field = fields.get(i + 1);

                let field_kind = match field.is_array_kind() {
                    true => "bytes_array",
                    false => "bytes",
                };
                write!(
                    out_file,
                    "decode_param!({0}; bytes, {1}, {1}_offset",
                    field_kind,
                    field.name.to_snake_case(),
                ).unwrap();
                if let Some(next_field) = next_field {
                    write!(out_file, ",{0}_offset", next_field.name.to_snake_case(),).unwrap();
                }
                write!(out_file, ");\n",).unwrap();
            }
            for field in fields.iter() {
                if field.required || field.is_array_kind() {
                    continue;
                }

                write!(
                    out_file,
                    "let {0}: Option<Vec<u8>> = to_option_bytes({0});\n",
                    field.name.to_snake_case()
                ).unwrap();
            }

            write!(
                out_file,
                "
                    Self {{
                ",
            ).unwrap();
            for field in fields.iter() {
                write!(out_file, "{0},\n", field.name.to_snake_case()).unwrap();
            }
            write!(
                out_file,
                "
                            }}
                        }}
                    }}
                ",
            ).unwrap();
        }
    }
}
