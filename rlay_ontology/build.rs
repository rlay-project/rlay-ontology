#![allow(unused_imports)]
extern crate heck;
extern crate proc_macro2;
extern crate prost_build;
#[macro_use]
extern crate quote;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate syn;

use std::env;
use std::fs::File;
use std::io::Write;
use std::io::prelude::*;
use std::path::Path;
use heck::SnakeCase;
use serde_json::Value;
use proc_macro2::TokenStream;

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

fn kind_names_types(kind_names: &[String]) -> Vec<syn::Type> {
    kind_names
        .iter()
        .map(|n| syn::parse_str(n).unwrap())
        .collect()
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
            // impl ::serde::Deserialize
            write_impl_deserialize(&mut out_file, kind_name, &fields);

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

    fn write_impl_serialize<W: Write>(writer: &mut W, kind_name: &str, fields: &[Field]) {
        write!(
            writer,
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
                write!(writer, "pub {0}: Vec<HexString<'a>>,\n", field.name).unwrap();
            } else {
                if field.required {
                    write!(writer, "pub {0}: HexString<'a>,\n", field.name).unwrap();
                } else {
                    write!(writer, "pub {0}: Option<HexString<'a>>,\n", field.name).unwrap();
                }
            }
        }
        write!(
            writer,
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
                    writer,
                    "{0}: self.{1}.iter().map(|n| HexString::wrap(n)).collect(),\n",
                    field.name,
                    field.name.to_snake_case(),
                ).unwrap();
            } else {
                if field.required {
                    write!(
                        writer,
                        "{0}: HexString::wrap(&self.{1}),\n",
                        field.name,
                        field.name.to_snake_case(),
                    ).unwrap();
                } else {
                    write!(
                        writer,
                        "{0}: HexString::wrap_option(self.{1}.as_ref()),\n",
                        field.name,
                        field.name.to_snake_case(),
                    ).unwrap();
                }
            }
        }
        write!(
            writer,
            "
                        }};

                        Ok(try!(ext.serialize(serializer)))
                    }}
                }}
            "
        ).unwrap();
    }

    fn write_impl_deserialize<W: Write>(writer: &mut W, kind_name: &str, fields: &[Field]) {
        write!(
            writer,
            "
                impl<'de> Deserialize<'de> for {0} {{
                    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                        where D: Deserializer<'de>,
                    {{
                        struct ThisEntityVisitor;
            ",
            kind_name
        ).unwrap();

        let field_names: Vec<String> = fields.iter().map(|n| n.name.clone()).collect();
        write!(
            writer,
            "const FIELDS: &'static [&'static str] = &{:?};",
            field_names
        ).unwrap();

        write!(
            writer,
            "
                impl<'de> Visitor<'de> for ThisEntityVisitor {{
                    type Value = {0};

                    fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {{
                        formatter.write_str(\"struct {0}\")
                    }}

                    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
                        where V: MapAccess<'de>,
                    {{
            ",
            kind_name
        ).unwrap();

        for field in fields.iter() {
            if field.is_array_kind() {
                write!(
                    writer,
                    "
                        let mut {0}: Option<Vec<String>> = None;
                    ",
                    field.name.to_snake_case()
                ).unwrap();
            } else {
                write!(
                    writer,
                    "
                        let mut {0}: Option<String> = None;
                    ",
                    field.name.to_snake_case()
                ).unwrap();
            }
        }

        write!(
            writer,
            "
                loop {{
                    let key = map.next_key::<String>()?;
                    match key {{
            "
        ).unwrap();

        for field in fields.iter() {
            write!(
                writer,
                "
                    Some(ref key) if key == \"{0}\" => {{
                        if {1}.is_some() {{
                            return Err(de::Error::duplicate_field(\"{0}\"));
                        }}
                        {1} = Some(map.next_value()?);
                    }}
                ",
                field.name,
                field.name.to_snake_case()
            ).unwrap();
        }

        write!(
            writer,
            "
                        Some(ref unknown) => {{
                            return Err(de::Error::unknown_field(unknown, FIELDS))
                        }}
                        None => break,
                    }}
                }}
            "
        ).unwrap();

        for field in fields.iter() {
            let field_name_raw = &field.name;
            let field_name_snake: syn::Ident = syn::parse_str(&field.name.to_snake_case()).unwrap();

            let field_deserialize_tokens: TokenStream = match field.is_array_kind() {
                true => {
                    parse_quote!{
                        let #field_name_snake = #field_name_snake
                            .unwrap_or(Vec::new())
                            .into_iter()
                            .map(|n| {{
                                n[2..].from_hex().map_err(|_| {{
                                    de::Error::invalid_value(
                                        de::Unexpected::Other("invalid hexstring"),
                                        &"hexstring",
                                    )
                                }})
                            }})
                            .collect::<Result<_, _>>()?;
                    }
                }
                false => {
                    let mut tokens = parse_quote!{
                        let #field_name_snake = #field_name_snake
                            .map(|n| {{
                                n[2..].from_hex().map_err(|_| {{
                                    de::Error::invalid_value(
                                        de::Unexpected::Other("invalid hexstring"),
                                        &"hexstring",
                                    )
                                }})
                            }}).map_or(Ok(None), |v| v.map(Some))?;
                    };
                    if field.required {
                        tokens = parse_quote!{
                            #tokens

                            let #field_name_snake = #field_name_snake.ok_or(de::Error::missing_field(#field_name_raw))?;
                        }
                    }
                    tokens
                }
            };
            write!(writer, "{}", field_deserialize_tokens).unwrap();
        }

        write!(
            writer,
            "
                Ok({0} {{
            ",
            kind_name
        ).unwrap();

        // Fields in constructor
        for field in fields.iter() {
            write!(
                writer,
                "
                    {0},
                ",
                field.name.to_snake_case()
            ).unwrap();
        }

        write!(
            writer,
            "
                        }})
                    }}
                }}
            ",
        ).unwrap();

        write!(
            writer,
            "
                        deserializer.deserialize_struct(\"{0}\", FIELDS, ThisEntityVisitor)
                    }}
                }}
            ",
            kind_name
        ).unwrap();
    }

    fn write_entity_kind<W: Write>(writer: &mut W, kind_names: Vec<String>) {
        write!(
            writer,
            "
            #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
            pub enum EntityKind {{
        "
        ).unwrap();
        for name in kind_names.iter() {
            write!(writer, "{},\n", name).unwrap();
        }
        write!(
            writer,
            "
            }}

            impl<'a> Into<&'a str> for EntityKind {{
                fn into(self) -> &'a str {{
                    match &self {{
        "
        ).unwrap();
        for name in kind_names.iter() {
            write!(writer, "EntityKind::{0} => \"{0}\",\n", name).unwrap();
        }
        write!(
            writer,
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
            write!(writer, "\"{0}\" => Ok(EntityKind::{0}),\n", name).unwrap();
        }
        write!(
            writer,
            "
                        _ => Err(()),
                    }}
                }}

                pub fn empty_entity(&self) -> Entity {{
                    match self {{
        "
        ).unwrap();
        for name in kind_names.iter() {
            write!(writer, "EntityKind::{0} => {0}::default().into(),\n", name).unwrap();
        }
        write!(
            writer,
            "
                    }}
                }}
            }}
        "
        ).unwrap();
    }

    fn write_entity<W: Write>(writer: &mut W, kind_names: Vec<String>) {
        let variants = kind_names_types(&kind_names);

        // Entity
        {
            let variants = variants.clone();
            let variants2 = variants.clone();
            let type_impl: TokenStream = parse_quote! {
                #[derive(Debug, Clone, PartialEq)]
                pub enum Entity {
                    #(#variants(#variants2)),
                    *
                }
            };
            write!(writer, "{}", type_impl).unwrap();
        }
        // impl ToCid
        {
            let variants = variants.clone();
            let trait_impl: TokenStream = parse_quote! {
                impl ToCid for Entity {
                    fn to_cid(&self) -> Result<Cid, CidError> {
                        match &self {
                            #(Entity::#variants(ent) => ent.to_cid()),
                            *
                        }
                    }
                }
            };
            write!(writer, "{}", trait_impl).unwrap();
        }
        // impl Entity
        {
            let variants = variants.clone();
            let variants2 = variants.clone();
            let type_impl: TokenStream = parse_quote! {
                impl Entity {
                    pub fn kind(&self) -> EntityKind {
                        match &self {
                            #(Entity::#variants(_) => EntityKind::#variants2),
                            *
                        }
                    }
                }
            };
            write!(writer, "{}", type_impl).unwrap();
        }
        // impl FromABIV2ResponseHinted
        {
            let variants = variants.clone();
            let variants2 = variants.clone();
            let trait_impl: TokenStream = parse_quote! {
                #[cfg(feature = "web3_compat")]
                impl FromABIV2ResponseHinted for Entity {
                    fn from_abiv2(bytes: &[u8], kind: &EntityKind) -> Self {
                        match kind {
                            #(EntityKind::#variants => Entity::#variants2(FromABIV2Response::from_abiv2(bytes))),
                            *
                        }
                    }
                }
            };
            write!(writer, "{}", trait_impl).unwrap();
        }
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

        // impl FromABIV2Response
        for raw_kind in kinds {
            write_entity_impl_from_abiv2_response(&mut out_file, raw_kind);
        }

        let kind_names: Vec<String> = kinds
            .iter()
            .map(|raw_kind| {
                let kind = raw_kind.as_object().unwrap();
                kind["name"].as_str().unwrap().to_owned()
            })
            .collect();
        write_entity_web3_format(&mut out_file, kind_names);
    }

    fn write_entity_impl_from_abiv2_response<W: Write>(writer: &mut W, raw_kind: &Value) {
        let kind = raw_kind.as_object().unwrap();
        let kind_name = kind["name"].as_str().unwrap();
        let fields: Vec<Field> = serde_json::from_value(kind["fields"].clone()).unwrap();

        let mut fn_body = std::io::Cursor::new(Vec::<u8>::new());

        for (i, field) in fields.iter().enumerate() {
            write!(
                fn_body,
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
                fn_body,
                "decode_param!({0}; bytes, {1}, {1}_offset",
                field_kind,
                field.name.to_snake_case(),
            ).unwrap();
            if let Some(next_field) = next_field {
                write!(fn_body, ",{0}_offset", next_field.name.to_snake_case(),).unwrap();
            }
            write!(fn_body, ");\n",).unwrap();
        }
        for field in fields.iter() {
            if field.required || field.is_array_kind() {
                continue;
            }

            write!(
                fn_body,
                "let {0}: Option<Vec<u8>> = to_option_bytes({0});\n",
                field.name.to_snake_case()
            ).unwrap();
        }

        write!(
            fn_body,
            "
                    Self {{
                ",
        ).unwrap();
        for field in fields.iter() {
            write!(fn_body, "{0},\n", field.name.to_snake_case()).unwrap();
        }
        write!(
            fn_body,
            "
                    }}
                ",
        ).unwrap();

        let fn_body_tokens: TokenStream =
            syn::parse_str(std::str::from_utf8(&fn_body.into_inner()).unwrap()).unwrap();
        let kind_name_ty: syn::Type = syn::parse_str(kind_name).unwrap();
        let trait_impl: TokenStream = parse_quote! {
            impl FromABIV2Response for #kind_name_ty {
                fn from_abiv2(bytes: &[u8]) -> Self {
                    #fn_body_tokens
                }
            }
        };
        write!(writer, "{}", trait_impl,).unwrap();
    }

    fn write_entity_web3_format<W: Write>(writer: &mut W, kind_names: Vec<String>) {
        let variants = kind_names_types(&kind_names);

        // EntityWeb3Format
        {
            let variants = variants.clone();
            let variants2 = variants.clone();
            let type_impl: TokenStream = parse_quote! {
                #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
                #[serde(tag = "type")]
                pub enum EntityWeb3Format {
                    #(#variants(#variants2)),
                    *
                }
            };
            write!(writer, "{}", type_impl).unwrap();
        }
        // From
        {
            let variants = variants.clone();
            let variants2 = variants.clone();
            let from_impl: TokenStream = parse_quote! {
                impl From<Entity> for EntityWeb3Format {
                    fn from(original: Entity) -> Self {
                        match original {
                            #(Entity::#variants(ent) => EntityWeb3Format::#variants2(ent)),
                            *
                        }
                    }
                }
            };
            write!(writer, "{}", from_impl).unwrap();
        }
        // Into
        {
            let variants = variants.clone();
            let variants2 = variants.clone();
            let into_impl: TokenStream = parse_quote! {
                impl Into<Entity> for EntityWeb3Format {
                    fn into(self) -> Entity {
                        match self {
                            #(EntityWeb3Format::#variants(ent) => Entity::#variants2(ent)),
                            *
                        }
                    }
                }
            };
            write!(writer, "{}", into_impl).unwrap();
        }
    }
}
