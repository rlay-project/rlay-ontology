#![allow(unused_imports)]
#![recursion_limit = "128"]
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

    pub fn field_ident(&self) -> syn::Ident {
        syn::parse_str(&self.name.to_snake_case()).unwrap()
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
        let field_names: Vec<String> = fields.iter().map(|n| n.name.clone()).collect();
        let field_names_const_decl: TokenStream = syn::parse_str(&format!(
            "const FIELDS: &'static [&'static str] = &{:?};",
            field_names
        )).unwrap();

        // intializes a empty Option variable for each field
        let initialize_empty_fields: TokenStream = fields
            .iter()
            .map(|field| {
                let field_ident = field.field_ident();
                let stmt: TokenStream = match field.is_array_kind() {
                    true => {
                        parse_quote! {
                            let mut #field_ident: Option<Vec<String>> = None;
                        }
                    }
                    false => {
                        parse_quote! {
                                    let mut #field_ident: Option<String> = None;
                        }
                    }
                };
                stmt
            })
            .collect();

        // tries to extract set the field variable if the field exists in the map
        let field_names_raw: Vec<String> = fields.iter().map(|n| n.name.clone()).collect();
        let field_names_raw2 = field_names_raw.clone();
        let field_names_snake: Vec<syn::Ident> = fields
            .iter()
            .map(|n| syn::parse_str(&n.name.to_snake_case()).unwrap())
            .collect();
        let field_names_snake2 = field_names_snake.clone();
        let extract_keys_loop: TokenStream = parse_quote! {
            loop {
                let key = map.next_key::<String>()?;
                match key {
                    #(
                        Some(ref key) if key == #field_names_raw => {
                            if #field_names_snake.is_some() {{
                                return Err(de::Error::duplicate_field(#field_names_raw2));
                            }}
                            #field_names_snake2 = Some(map.next_value()?);
                        }
                     )*
                    Some(ref unknown) => {
                        return Err(de::Error::unknown_field(unknown, FIELDS))
                    }
                    None => break,
                }
            }
        };

        // applies appropiate deserialize call to each field accoring to type
        let field_deserialize_calls: TokenStream = fields
            .iter()
            .map(|field| {
                let field_name_raw = &field.name;
                let field_name_snake: syn::Ident =
                    syn::parse_str(&field.name.to_snake_case()).unwrap();

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
                field_deserialize_tokens
            })
            .collect();

        let kind_ty: syn::Type = syn::parse_str(kind_name).unwrap();
        let field_idents: Vec<_> = fields.iter().map(|n| n.field_ident()).collect();
        let constructor_call: TokenStream = parse_quote! {
            Ok(#kind_ty {
                #(#field_idents),
                *
            })
        };

        let expecting_msg = format!("struct {}", kind_name);
        let trait_impl: TokenStream = parse_quote! {
            impl<'de> Deserialize<'de> for #kind_ty {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where D: Deserializer<'de>,
                {
                    struct ThisEntityVisitor;

                    #field_names_const_decl

                    impl<'de> Visitor<'de> for ThisEntityVisitor {
                        type Value = #kind_ty;

                        fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                            formatter.write_str(#expecting_msg)
                        }

                        fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
                            where V: MapAccess<'de>,
                        {
                            #initialize_empty_fields
                            #extract_keys_loop
                            #field_deserialize_calls
                            #constructor_call

                        }
                    }
                    deserializer.deserialize_struct(#kind_name, FIELDS, ThisEntityVisitor)
                }
            }
        };

        write!(writer, "{}", trait_impl).unwrap();
    }

    fn write_entity_kind<W: Write>(writer: &mut W, kind_names: Vec<String>) {
        let variants = kind_names_types(&kind_names);
        // EntityKind
        {
            let variants = variants.clone();
            let type_impl: TokenStream = parse_quote! {
                #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
                pub enum EntityKind {
                    #(#variants),
                    *
                }
            };
            write!(writer, "{}", type_impl).unwrap();
        }
        // Into<&'a str>
        {
            let variants = variants.clone();
            let kind_names = kind_names.clone();
            let trait_impl: TokenStream = parse_quote! {
                impl<'a> Into<&'a str> for EntityKind {
                    fn into(self) -> &'a str {
                        match &self {
                            #(EntityKind::#variants => #kind_names),
                            *
                        }
                    }
                }
            };
            write!(writer, "{}", trait_impl).unwrap();
        }
        // impl EntityKind
        {
            let kind_names = kind_names.clone();
            let variants = variants.clone();
            let variants2 = variants.clone();
            let variants3 = variants.clone();
            let trait_impl: TokenStream = parse_quote! {
                impl EntityKind {
                    pub fn from_name(name: &str) -> Result<Self, ()> {
                        match name {
                            #(#kind_names => Ok(EntityKind::#variants)),*,
                            _ => Err(()),
                        }
                    }

                    pub fn empty_entity(&self) -> Entity {
                        match self {
                            #(EntityKind::#variants2 => #variants3::default().into()),*
                        }
                    }
                }
            };
            write!(writer, "{}", trait_impl).unwrap();
        }
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

        let decode_offset_macros: TokenStream = fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let offset_ident: syn::Ident =
                    syn::parse_str(&format!("{}_offset", field.name.to_snake_case())).unwrap();
                let offset_start = i * 32;
                let offset_end = (i + 1) * 32;
                let tokens: TokenStream = parse_quote! {
                    decode_offset!(bytes, #offset_ident, #offset_start, #offset_end);
                };
                tokens
            })
            .collect();

        let decode_param_macros: TokenStream = fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let field_kind_marker: syn::Ident = syn::parse_str(match field.is_array_kind() {
                    true => "bytes_array",
                    false => "bytes",
                }).unwrap();

                let field_ident = field.field_ident();
                let offset_ident: syn::Ident =
                    syn::parse_str(&format!("{}_offset", field.name.to_snake_case())).unwrap();

                let next_field = fields.get(i + 1);
                let tokens: TokenStream = match next_field {
                    Some(next_field) => {
                        let next_offset_ident: syn::Ident =
                            syn::parse_str(&format!("{}_offset", next_field.name.to_snake_case()))
                                .unwrap();
                        parse_quote! {
                            decode_param!(#field_kind_marker; bytes, #field_ident, #offset_ident, #next_offset_ident);
                        }
                    }
                    None => {
                        parse_quote! {
                            decode_param!(#field_kind_marker; bytes, #field_ident, #offset_ident);
                        }
                    }
                };
                tokens
            })
            .collect();

        let wrap_option_fields: TokenStream = fields
            .iter()
            .filter_map(|field| {
                if field.required || field.is_array_kind() {
                    return None;
                }
                let field_ident = field.field_ident();
                let tokens: TokenStream = parse_quote! {
                    let #field_ident: Option<Vec<u8>> = to_option_bytes(#field_ident);
                };
                Some(tokens)
            })
            .collect();

        let field_idents: Vec<_> = fields.iter().map(|n| n.field_ident()).collect();
        let constructor: TokenStream = parse_quote! {
            Self {
                #(#field_idents),*
            }
        };

        let kind_name_ty: syn::Type = syn::parse_str(kind_name).unwrap();
        let trait_impl: TokenStream = parse_quote! {
            impl FromABIV2Response for #kind_name_ty {
                fn from_abiv2(bytes: &[u8]) -> Self {
                    #decode_offset_macros
                    #decode_param_macros
                    #wrap_option_fields

                    #constructor
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
