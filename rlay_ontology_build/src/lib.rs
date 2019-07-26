#![allow(unused_imports)]
#![recursion_limit = "256"]
extern crate heck;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate syn;

#[cfg(feature = "serde_json")]
extern crate serde_json;

mod intermediate;

use heck::SnakeCase;
use proc_macro2::TokenStream;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::Write;
use std::path::Path;

use intermediate::{parse_intermediate_contents, Field, Kind};

pub fn build_files() {
    entities::build_file("src/intermediate.json", "rlay.ontology.entities.rs");
    main::build_macros_applied_file("src/intermediate.json", "rlay.ontology.macros_applied.rs");
    web3::build_applied_file("src/intermediate.json", "rlay.ontology.web3_applied.rs");
    compact::build_file("src/intermediate.json", "rlay.ontology.compact.rs");
    v0::build_file("src/intermediate.json", "rlay.ontology.v0.rs");
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
        let intermediate = parse_intermediate_contents(&intermediate_contents);

        let mut out_file = File::create(&dest_path).unwrap();

        let kinds = intermediate.kinds;
        for raw_kind in kinds.iter() {
            let kind_name = &raw_kind.name;
            let kind_cid_prefix = raw_kind.cidPrefix;

            // Header line
            write!(out_file, "\n// {}\n", kind_name).unwrap();
            // impl AssociatedCodec
            write!(
                out_file,
                "codec_code!({}, {});\n",
                kind_name, kind_cid_prefix
            )
            .unwrap();
            // impl ToCid
            let kind_ty: syn::Type = syn::parse_str(kind_name).unwrap();
            let impl_to_cid: TokenStream = parse_quote! {
                #[cfg(feature = "std")]
                impl_to_cid!(#kind_ty);
            };
            write!(out_file, "{}", impl_to_cid).unwrap();
            // impl Canonicalize
            {
                let kind_ty: syn::Type = syn::parse_str(kind_name).unwrap();
                let fields: Vec<syn::Ident> = raw_kind
                    .fields
                    .clone()
                    .into_iter()
                    .filter(|n| n.is_array_kind())
                    .map(|n| n.field_ident())
                    .collect();
                let impl_canonicalize: TokenStream = parse_quote! {
                    impl_canonicalize!(#kind_ty; #(#fields),*);
                };
                write!(out_file, "{}", impl_canonicalize).unwrap();
            }
            // impl CidFields
            write_impl_cid_fields(&mut out_file, kind_name, &raw_kind.fields);

            write!(
                out_file,
                "impl_into_entity_kind!({0}, Entity::{0});\n",
                kind_name
            )
            .unwrap();
        }

        let kind_names: Vec<String> = kinds
            .iter()
            .map(|raw_kind| raw_kind.name.to_owned())
            .collect();
        let kind_ids: Vec<u64> = kinds.iter().map(|raw_kind| raw_kind.kindId).collect();
        write_entity_kind(&mut out_file, kind_names.clone(), kind_ids.clone());
        write_entity(&mut out_file, kind_names.clone());
    }

    fn write_impl_cid_fields<W: Write>(writer: &mut W, kind_name: &str, fields: &[Field]) {
        let mut fields = fields.to_owned();
        fields = fields
            .into_iter()
            .filter(|field| {
                if kind_name == "Annotation" && field.name == "value" {
                    return false;
                }
                if kind_name == "DataPropertyAssertion" && field.name == "target" {
                    return false;
                }
                if kind_name == "NegativeDataPropertyAssertion" && field.name == "target" {
                    return false;
                }
                true
            })
            .collect();

        let kind_ty: syn::Type = syn::parse_str(kind_name).unwrap();
        let iter_struct_name: syn::Type =
            syn::parse_str(&format!("{}CidFields", kind_name)).unwrap();
        let iter_struct: TokenStream = parse_quote! {
            pub struct #iter_struct_name<'a> {
                inner: &'a #kind_ty,
                field_index: usize,
                field_vec_index: usize,
            }
        };
        write!(writer, "{}", iter_struct).unwrap();

        let kind_ty: syn::Type = syn::parse_str(kind_name).unwrap();
        let iter_struct_name: syn::Type =
            syn::parse_str(&format!("{}CidFields", kind_name)).unwrap();
        let iter_struct_impl: TokenStream = parse_quote! {
            impl<'a> #iter_struct_name<'a> {
                fn new(inner: &'a #kind_ty) -> Self {
                    Self {
                        inner,
                        field_index: 0,
                        field_vec_index: 0,
                    }
                }
            }
        };
        write!(writer, "{}", iter_struct_impl).unwrap();

        let iter_blocks: Vec<TokenStream> = fields
            .iter()
            .map(|field| {
                let field_ident = field.field_ident();
                let stmt: TokenStream = match (field.is_array_kind(), field.required) {
                    (true, _) => parse_quote! {
                        item = self.inner.#field_ident.get(self.field_vec_index);
                        self.field_vec_index += 1;
                        if self.inner.#field_ident.len() <= self.field_vec_index {
                            self.field_vec_index = 0;
                            self.field_index += 1;
                        }
                    },
                    (false, true) => parse_quote! {
                        item = Some(&self.inner.#field_ident);
                        self.field_index += 1;
                    },
                    (false, false) => parse_quote! {
                        item = self.inner.#field_ident.as_ref();
                        self.field_index += 1;
                    },
                };
                stmt
            })
            .collect();

        let iter_struct_name: syn::Type =
            syn::parse_str(&format!("{}CidFields", kind_name)).unwrap();
        let field_indices: Vec<_> = (0..fields.len()).collect();
        let iter_struct_impl_iterator: TokenStream = parse_quote! {
            impl<'a> Iterator for #iter_struct_name<'a> {
                type Item = &'a Vec<u8>;

                fn next(&mut self) -> Option<Self::Item> {
                    let mut item = None;

                    #(
                    if item == None && self.field_index == #field_indices {
                        #iter_blocks
                    }
                    )*

                    item
                }
            }
        };
        write!(writer, "{}", iter_struct_impl_iterator).unwrap();

        let kind_ty: syn::Type = syn::parse_str(kind_name).unwrap();
        let iter_struct_name: syn::Type =
            syn::parse_str(&format!("{}CidFields", kind_name)).unwrap();
        let impl_cid_fields: TokenStream = parse_quote! {
            impl<'a> CidFields<'a> for #kind_ty {
                type Iter = #iter_struct_name<'a>;

                fn iter_cid_fields(&'a self) -> #iter_struct_name {
                    #iter_struct_name::new(self)
                }
            }
        };
        write!(writer, "{}", impl_cid_fields).unwrap();
    }

    fn write_entity_kind<W: Write>(writer: &mut W, kind_names: Vec<String>, kind_ids: Vec<u64>) {
        let variants = kind_names_types(&kind_names);
        // EntityKind
        {
            let variants = variants.clone();
            let type_impl: TokenStream = parse_quote! {
                #[derive(Debug, Clone, PartialEq)]
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
            let variants2 = variants.clone();
            let kind_names = kind_names.clone();
            let kind_names2 = kind_names.clone();
            let trait_impl: TokenStream = parse_quote! {
                impl<'a> Into<&'a str> for EntityKind {
                    fn into(self) -> &'a str {
                        match &self {
                            #(EntityKind::#variants => #kind_names),
                            *
                        }
                    }
                }

                impl<'a> Into<&'a str> for &'a EntityKind {
                    fn into(self) -> &'a str {
                        match &self {
                            #(EntityKind::#variants2 => #kind_names2),
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
            let kind_ids = kind_ids.clone();
            let variants = variants.clone();
            let variants2 = variants.clone();
            let variants3 = variants.clone();
            let variants4 = variants.clone();
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

                    pub fn id(&self) -> u64 {
                        match self {
                            #(EntityKind::#variants4 => #kind_ids),*
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
                #[cfg(feature = "std")]
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
        // impl Canonicalize
        {
            let variants = variants.clone();
            let trait_impl: TokenStream = parse_quote! {
                impl Canonicalize for Entity {
                    fn canonicalize(&mut self) {
                        match self {
                            #(Entity::#variants(ref mut ent) => ent.canonicalize()),
                            *
                        }
                    }
                }
            };
            write!(writer, "{}", trait_impl).unwrap();
        }
        // impl CidFields
        {
            let variants2 = variants.clone();
            let variants_iter_structs: Vec<syn::Type> = kind_names
                .clone()
                .into_iter()
                .map(|variant| syn::parse_str(&format!("{}CidFields", variant)).unwrap())
                .collect();

            let enum_impl: TokenStream = parse_quote! {
                pub enum EntityCidFields<'a> {
                    #(#variants2(#variants_iter_structs<'a>)),
                    *
                }
            };
            write!(writer, "{}", enum_impl).unwrap();

            let variants2 = variants.clone();
            let enum_impl_iterator: TokenStream = parse_quote! {
                impl<'a> Iterator for EntityCidFields<'a> {
                    type Item = &'a Vec<u8>;

                    fn next(&mut self) -> Option<Self::Item> {
                        match self {
                            #(EntityCidFields::#variants2(inner) => inner.next()),
                            *
                        }
                    }
                }
            };
            write!(writer, "{}", enum_impl_iterator).unwrap();

            let variants2 = variants.clone();
            let variants3 = variants.clone();
            let trait_impl: TokenStream = parse_quote! {
                impl<'a> CidFields<'a> for Entity {
                    type Iter = EntityCidFields<'a>;

                    fn iter_cid_fields(&'a self) -> EntityCidFields {
                        match self {
                            #(Entity::#variants2(inner) => EntityCidFields::#variants3(inner.iter_cid_fields())),
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

mod entities {
    use super::*;

    pub fn build_file(src_path: &str, out_path: &str) {
        let out_dir = env::var("OUT_DIR").unwrap();
        let dest_path = Path::new(&out_dir).join(out_path);

        let mut intermediate_file = File::open(src_path).expect("file not found");

        let mut intermediate_contents = String::new();
        intermediate_file
            .read_to_string(&mut intermediate_contents)
            .unwrap();
        let intermediate = parse_intermediate_contents(&intermediate_contents);

        let mut out_file = File::create(&dest_path).unwrap();

        let kinds = intermediate.kinds;
        for raw_kind in kinds.iter() {
            let kind_name = &raw_kind.name;
            let fields: Vec<_> = raw_kind.fields.clone();

            write_entity(&mut out_file, kind_name, &fields);
        }
    }

    fn write_entity<W: Write>(writer: &mut W, kind_name: &str, fields: &[Field]) {
        let fields: TokenStream = fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let field_ident = field.field_ident();
                let i_str = (i + 1).to_string();
                let prost_attribute: TokenStream = match (field.is_array_kind(), field.required) {
                    (true, _) => parse_quote!(prost(bytes, repeated, tag=#i_str)),
                    (false, true) => parse_quote!(prost(bytes, required, tag=#i_str)),
                    (false, false) => parse_quote!(prost(bytes, optional, tag=#i_str)),
                };
                let field_ty: syn::Type = match (field.is_array_kind(), field.required) {
                    (true, _) => parse_quote!(Vec<Vec<u8>>),
                    (false, true) => parse_quote!(Vec<u8>),
                    (false, false) => parse_quote!(Option<Vec<u8>>),
                };
                let tokens: TokenStream = parse_quote! {
                    #[cfg_attr(feature = "std", #prost_attribute)]
                    pub #field_ident: #field_ty,
                };
                tokens
            })
            .collect();

        let entity_ty: syn::Type = syn::parse_str(kind_name).unwrap();
        let entity_struct: TokenStream = parse_quote! {
            #[derive(Clone, PartialEq)]
            #[cfg_attr(not(feature = "std"), derive(Debug, Default))]
            #[cfg_attr(feature = "std", derive(Message))]
            pub struct #entity_ty {
                #fields
            }
        };
        write!(writer, "{}", entity_struct).unwrap();
    }
}

fn write_format_variant_wrapper<W: Write>(
    writer: &mut W,
    format_suffix: &str,
    kind_name: &str,
    _fields: &[Field],
    write_conversion_trait: bool,
) {
    // Wrapper
    let wrapper_ty: syn::Type =
        syn::parse_str(&format!("{}Format{}", kind_name, format_suffix)).unwrap();
    let inner_ty: syn::Type = syn::parse_str(kind_name).unwrap();
    let wrapper_struct: TokenStream = parse_quote! {
        #[cfg_attr(feature = "wasm_bindgen", wasm_bindgen)]
        #[derive(Debug, Clone, PartialEq, Default)]
        pub struct #wrapper_ty {
            inner: #inner_ty
        }
    };
    write!(writer, "{}", wrapper_struct).unwrap();
    // From
    {
        let trait_impl: TokenStream = parse_quote! {
            impl From<#inner_ty> for #wrapper_ty {
                fn from(original: #inner_ty) -> Self {
                    Self {
                        inner: original
                    }
                }
            }
        };
        write!(writer, "{}", trait_impl).unwrap();
    }
    // Into
    {
        let trait_impl: TokenStream = parse_quote! {
            impl Into<#inner_ty> for #wrapper_ty {
                fn into(self) -> #inner_ty {
                    self.inner
                }
            }
        };
        write!(writer, "{}", trait_impl).unwrap();
    }
    if write_conversion_trait {
        let conversion_trait: syn::Type =
            syn::parse_str(&format!("Format{}", format_suffix)).unwrap();
        let format_suffix_lc = format_suffix.to_lowercase();
        let to_fn_ident: syn::Ident =
            syn::parse_str(&format!("to_{}_format", format_suffix_lc)).unwrap();
        let from_fn_ident: syn::Ident =
            syn::parse_str(&format!("from_{}_format", format_suffix_lc)).unwrap();

        let trait_impl: TokenStream = parse_quote! {
            #[cfg(feature = "std")]
            impl<'a> #conversion_trait<'a> for #inner_ty {
                type Formatted = #wrapper_ty;
                fn #to_fn_ident(self) -> Self::Formatted {
                    #wrapper_ty::from(self)
                }

                fn #from_fn_ident(formatted: Self::Formatted) -> Self {
                    formatted.into()
                }
            }
        };
        write!(writer, "{}", trait_impl).unwrap();
    }
}

mod compact {
    use super::*;

    pub fn build_file(src_path: &str, out_path: &str) {
        let out_dir = env::var("OUT_DIR").unwrap();
        let dest_path = Path::new(&out_dir).join(out_path);

        let mut intermediate_file = File::open(src_path).expect("file not found");

        let mut intermediate_contents = String::new();
        intermediate_file
            .read_to_string(&mut intermediate_contents)
            .unwrap();
        let intermediate = parse_intermediate_contents(&intermediate_contents);

        let mut out_file = File::create(&dest_path).unwrap();

        let kinds = intermediate.kinds;
        for raw_kind in kinds {
            let kind_name = &raw_kind.name;
            let fields = raw_kind.fields.clone();

            write_variant_format_compact(&mut out_file, kind_name, &fields);
        }
    }

    fn write_variant_format_compact<W: Write>(writer: &mut W, kind_name: &str, fields: &[Field]) {
        write_format_variant_wrapper(writer, "Compact", kind_name, fields, true);
        write_format_compact_impl_serialize(writer, kind_name, fields);
        write_format_compact_impl_deserialize(writer, kind_name, fields);
    }

    fn write_format_compact_impl_serialize<W: Write>(
        writer: &mut W,
        kind_name: &str,
        fields: &[Field],
    ) {
        let helper_fields: TokenStream = fields
            .iter()
            .map(|field| {
                let field_ident = field.field_ident();
                let tokens: TokenStream = match (field.is_array_kind(), field.required) {
                    (true, _) => parse_quote! {
                        #[serde(skip_serializing_if = "Vec::is_empty")]
                        // TODO: bytes serialize
                        pub #field_ident: &'a Vec<Vec<u8>>,
                    },
                    (false, true) => parse_quote! {
                        #[serde(with = "serde_bytes")]
                        pub #field_ident: &'a Vec<u8>,
                    },
                    (false, false) => parse_quote! {
                        #[serde(skip_serializing_if = "Option::is_none")]
                        // TODO: bytes serialize
                        pub #field_ident: &'a Option<Vec<u8>>,
                    },
                };
                tokens
            })
            .collect();

        let wrap_helper_fields: TokenStream = fields
            .iter()
            .map(|field| {
                let field_ident = field.field_ident();
                let tokens: TokenStream = parse_quote!(#field_ident: &self.inner.#field_ident,);
                tokens
            })
            .collect();

        let wrapper_ty: syn::Type = syn::parse_str(&format!("{}FormatCompact", kind_name)).unwrap();
        let trait_impl: TokenStream = parse_quote! {
            #[cfg(feature = "std")]
            impl ::serde::Serialize for #wrapper_ty {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: ::serde::Serializer,
                {
                    #[derive(Serialize)]
                    #[allow(non_snake_case)]
                    struct SerializeHelper<'a> {
                        #helper_fields
                    }

                    let ext = SerializeHelper {
                        #wrap_helper_fields
                    };

                    Ok(ext.serialize(serializer)?)
                }
            }
        };
        write!(writer, "{}", trait_impl).unwrap();
    }

    fn write_format_compact_impl_deserialize<W: Write>(
        writer: &mut W,
        kind_name: &str,
        fields: &[Field],
    ) {
        let helper_fields: TokenStream = fields
            .iter()
            .map(|field| {
                let field_ident = field.field_ident();
                let stmt: TokenStream = match (field.is_array_kind(), field.required) {
                    (true, _) => parse_quote! {
                        #[serde(default, deserialize_with = "nullable_vec")]
                        #field_ident: Vec<Vec<u8>>,
                    },
                    (false, true) => parse_quote! {
                        #[serde(with = "serde_bytes")]
                        #field_ident: Vec<u8>,
                    },
                    (false, false) => parse_quote! {
                        #[serde(default)]
                        // TODO: bytes serialize
                        #field_ident: Option<Vec<u8>>,
                    },
                };
                stmt
            })
            .collect();

        let kind_ty: syn::Type = syn::parse_str(kind_name).unwrap();
        let wrapper_ty: syn::Type = syn::parse_str(&format!("{}FormatCompact", kind_name)).unwrap();
        let field_idents: Vec<_> = fields.iter().map(|n| n.field_ident()).collect();
        let field_idents2 = field_idents.clone();
        let constructor_call: TokenStream = parse_quote! {
            Ok(#wrapper_ty {
                inner: #kind_ty {
                    #(#field_idents: helper_instance.#field_idents2),
                    *
                }
            })
        };

        let trait_impl: TokenStream = parse_quote! {
            #[cfg(feature = "std")]
            impl<'de> Deserialize<'de> for #wrapper_ty {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where D: Deserializer<'de>,
                {
                    #[derive(Deserialize)]
                    struct DeserializeHelper {
                        #helper_fields
                    }

                    fn nullable_vec<'de, D>(deserializer: D) -> Result<Vec<Vec<u8>>, D::Error>
                        where D: Deserializer<'de>
                    {
                        let opt: Option<Vec<serde_bytes::ByteBuf>> = Option::deserialize(deserializer)?;
                        let val = opt
                            .unwrap_or_else(Vec::new)
                            .into_iter()
                            .map(|n| (*n).to_vec())
                            .collect();
                        Ok(val)
                    }

                    let helper_instance = DeserializeHelper::deserialize(deserializer)?;
                    #constructor_call
                }
            }
        };

        write!(writer, "{}", trait_impl).unwrap();
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
        let intermediate = parse_intermediate_contents(&intermediate_contents);

        let mut out_file = File::create(&dest_path).unwrap();

        let kinds = intermediate.kinds;
        // impl FromABIV2Response
        for raw_kind in kinds.iter() {
            let kind_name = &raw_kind.name;
            let fields = raw_kind.fields.clone();

            write_entity_impl_from_abiv2_response(&mut out_file, raw_kind);
            write_variant_format_web3(&mut out_file, kind_name, &fields);
        }

        let kind_names: Vec<String> = kinds
            .iter()
            .map(|raw_kind| raw_kind.name.to_owned())
            .collect();
        write_entity_format_web3(&mut out_file, kind_names);
    }

    fn write_entity_impl_from_abiv2_response<W: Write>(writer: &mut W, raw_kind: &Kind) {
        let kind_name = &raw_kind.name;
        let fields = raw_kind.fields.clone();

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

    fn write_entity_format_web3<W: Write>(writer: &mut W, kind_names: Vec<String>) {
        let variants = kind_names_types(&kind_names);
        let wrapper_variants: Vec<syn::Type> = kind_names
            .iter()
            .map(|n| syn::parse_str(&format!("FormatWeb3<{}>", n)).unwrap())
            .collect();

        // SerializeFormatWeb3 for Entity
        {
            let variants = variants.clone();
            let variants2 = variants.clone();
            let variants3 = variants.clone();
            let wrapper_variants = wrapper_variants.clone();
            let type_impl: TokenStream = parse_quote! {
                impl SerializeFormatWeb3 for Entity {
                    fn serialize_format_web3<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                        where
                        S: serde::Serializer,
                    {
                        #[derive(Serialize)]
                        #[serde(tag = "type")]
                        pub enum EntityFormatWeb3 {
                            #(#variants(#wrapper_variants)),
                            *
                        }

                        impl Into<EntityFormatWeb3> for Entity {
                            fn into(self) -> EntityFormatWeb3 {
                                match self {
                                    #(Entity::#variants3(ent) => EntityFormatWeb3::#variants2(ent.into())),
                                    *
                                }
                            }
                        }

                        let proxy: EntityFormatWeb3 = self.to_owned().into();
                        proxy.serialize(serializer)
                    }
                }
            };
            write!(writer, "{}", type_impl).unwrap();
        }
        // DeserializeFormatWeb3 for Entity
        {
            let variants = variants.clone();
            let variants2 = variants.clone();
            let variants3 = variants.clone();
            let wrapper_variants = wrapper_variants.clone();
            let type_impl: TokenStream = parse_quote! {
                impl<'de> DeserializeFormatWeb3<'de> for Entity {
                    fn deserialize_format_web3<D>(deserializer: D) -> Result<Self, D::Error>
                    where
                        D: Deserializer<'de>,
                    {
                        #[derive(Deserialize)]
                        #[serde(tag = "type")]
                        pub enum EntityFormatWeb3 {
                            #(#variants(#wrapper_variants)),
                            *
                        }

                        impl From<EntityFormatWeb3> for Entity {
                            fn from(original: EntityFormatWeb3) -> Entity {
                                match original {
                                    #(EntityFormatWeb3::#variants3(ent) => Entity::#variants2(ent.0)),
                                    *
                                }
                            }
                        }

                        let deserialized = EntityFormatWeb3::deserialize(deserializer)?;
                        Ok(deserialized.into())
                    }
                }
            };
            write!(writer, "{}", type_impl).unwrap();
        }
    }

    fn write_variant_format_web3<W: Write>(writer: &mut W, kind_name: &str, fields: &[Field]) {
        write_format_web3_impl_serialize_format_web3(writer, kind_name, fields);
        write_format_web3_impl_deserialize_format_web3(writer, kind_name, fields);
    }

    fn write_format_web3_impl_serialize_format_web3<W: Write>(
        writer: &mut W,
        kind_name: &str,
        fields: &[Field],
    ) {
        let kind_type: syn::Type = syn::parse_str(kind_name).unwrap();
        // +1 for "cid" field
        let field_num = fields.len() + 1;
        let field_idents: Vec<syn::Ident> =
            fields.iter().map(|field| field.field_ident()).collect();
        let field_names: Vec<String> = fields.iter().map(|n| n.name.to_string()).collect();
        let trait_impl: TokenStream = parse_quote! {
            impl SerializeFormatWeb3 for #kind_type {
                fn serialize_format_web3<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    let mut s = serializer.serialize_struct(#kind_name, #field_num)?;
                    s.serialize_field("cid", &self.to_cid().ok().map(|n| FormatWeb3(n.to_bytes())))?;
                    #(s.serialize_field(#field_names, &FormatWeb3(&self.#field_idents))?;)*

                    s.end()
                }
            }
        };

        write!(writer, "{}", trait_impl).unwrap();
    }

    fn write_format_web3_impl_deserialize_format_web3<W: Write>(
        writer: &mut W,
        kind_name: &str,
        fields: &[Field],
    ) {
        let field_names: Vec<String> = fields.iter().map(|n| n.name.clone()).collect();
        let field_names_const_decl: TokenStream = syn::parse_str(&format!(
            "const FIELDS: &'static [&'static str] = &{:?};",
            field_names
        ))
        .unwrap();

        // intializes a empty Option variable for each field
        let initialize_empty_fields: TokenStream = fields
            .iter()
            .map(|field| {
                let field_ident = field.field_ident();
                let stmt: TokenStream = parse_quote!(let mut #field_ident: Option<_> = None;);
                stmt
            })
            .collect();

        // tries to extract set the field variable if the field exists in the map
        let field_names_raw: Vec<String> = fields.iter().map(|n| n.name.clone()).collect();
        let field_names_raw2 = field_names_raw.clone();
        let field_names_snake: Vec<syn::Ident> = fields.iter().map(|n| n.field_ident()).collect();
        let extract_key_blocks: Vec<TokenStream> = fields
            .iter()
            .map(|field| {
                let field_ident = field.field_ident();
                let stmt: TokenStream = match (field.is_array_kind(), field.required) {
                    (true, _) => parse_quote! {
                        let inner_val: Vec<FormatWeb3<Vec<u8>>> = map.next_value()?;
                        let inner_val: Vec<Vec<u8>> = inner_val.into_iter().map(|n| n.0).collect();
                        #field_ident = Some(inner_val);
                    },
                    (false, true) => parse_quote! {
                        let inner_val: FormatWeb3<Vec<u8>> = map.next_value()?;
                        #field_ident = Some(inner_val.0);
                    },
                    (false, false) => parse_quote! {
                        let inner_val: Option<FormatWeb3<Vec<u8>>> = map.next_value()?;
                        #field_ident = Some(inner_val.map(|n| n.0));
                    },
                };
                stmt
            })
            .collect();
        let extract_keys_loop: TokenStream = parse_quote! {
            loop {
                let key = map.next_key::<String>()?;
                match key {
                    #(
                        Some(ref key) if key == #field_names_raw => {
                            if #field_names_snake.is_some() {{
                                return Err(de::Error::duplicate_field(#field_names_raw2));
                            }}
                            #extract_key_blocks
                        }
                     )*
                    Some(ref unknown) => {
                        return Err(de::Error::unknown_field(unknown, FIELDS))
                    }
                    None => break,
                }
            }
        };

        let enforce_required_fields: Vec<TokenStream> = fields
            .iter()
            .map(|field| {
                let field_name_raw = &field.name;
                let field_ident = field.field_ident();
                let stmt: TokenStream = match field.required {
                    true => parse_quote! {
                        let #field_ident = #field_ident.ok_or(de::Error::missing_field(#field_name_raw))?;
                    },
                    false => parse_quote! {
                        let #field_ident = #field_ident.unwrap_or_default();
                    },
                };
                stmt
            })
            .collect();
        let enforce_required_fields: TokenStream = parse_quote! {
            #(#enforce_required_fields)
            *
        };

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
            impl<'de> DeserializeFormatWeb3<'de> for #kind_ty {
                fn deserialize_format_web3<D>(deserializer: D) -> Result<Self, D::Error>
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
                            #enforce_required_fields
                            #constructor_call

                        }
                    }
                    deserializer.deserialize_struct(#kind_name, FIELDS, ThisEntityVisitor)
                }
            }
        };

        write!(writer, "{}", trait_impl).unwrap();
    }
}

mod v0 {
    use super::*;

    pub fn build_file(src_path: &str, out_path: &str) {
        let out_dir = env::var("OUT_DIR").unwrap();
        let dest_path = Path::new(&out_dir).join(out_path);

        let mut intermediate_file = File::open(src_path).expect("file not found");

        let mut intermediate_contents = String::new();
        intermediate_file
            .read_to_string(&mut intermediate_contents)
            .unwrap();
        let intermediate = parse_intermediate_contents(&intermediate_contents);

        let mut out_file = File::create(&dest_path).unwrap();

        let kinds = intermediate.kinds;
        let kind_names: Vec<String> = kinds
            .iter()
            .map(|raw_kind| raw_kind.name.to_owned())
            .collect();
        let kind_ids: Vec<u64> = kinds.iter().map(|raw_kind| raw_kind.kindId).collect();

        write_entity(&mut out_file, kind_names, kind_ids);
    }

    fn write_entity<W: Write>(writer: &mut W, kind_names: Vec<String>, kind_ids: Vec<u64>) {
        let variants = kind_names_types(&kind_names);

        // Entity
        {
            let variants = variants.clone();
            let variants2 = variants.clone();
            let type_impl: TokenStream = parse_quote! {
                #[derive(Debug, Clone, PartialEq)]
                pub enum EntityV0 {
                    #(#variants(#variants2)),
                    *
                }
            };
            write!(writer, "{}", type_impl).unwrap();
        }
        // impl ToCid
        // {
        // let variants = variants.clone();
        // let trait_impl: TokenStream = parse_quote! {
        // impl ToCid for Entity {
        // fn to_cid(&self) -> Result<Cid, CidError> {
        // match &self {
        // #(Entity::#variants(ent) => ent.to_cid()),
        // *
        // }
        // }
        // }
        // };
        // write!(writer, "{}", trait_impl).unwrap();
        // }
        // impl Into<Entity>
        {
            let variants = variants.clone();
            let variants2 = variants.clone();
            let type_impl: TokenStream = parse_quote! {
                impl Into<Entity> for EntityV0 {
                    fn into(self) -> Entity {
                        match self {
                            #(EntityV0::#variants(ent) => Entity::#variants2(ent)),
                            *
                        }
                    }
                }
            };
            write!(writer, "{}", type_impl).unwrap();
        }
        // impl Into<EntityV0>
        {
            let variants = variants.clone();
            let variants2 = variants.clone();
            let type_impl: TokenStream = parse_quote! {
                impl Into<EntityV0> for Entity {
                    fn into(self) -> EntityV0 {
                        match self {
                            #(Entity::#variants(ent) => EntityV0::#variants2(ent)),
                            *
                        }
                    }
                }
            };
            write!(writer, "{}", type_impl).unwrap();
        }
        // impl EntityV0
        {
            let variants = variants.clone();
            let variants2 = variants.clone();
            let trait_impl: TokenStream = parse_quote! {
                impl EntityV0 {
                    #[cfg(feature = "std")]
                    pub fn serialize<W: ::std::io::Write>(&self, writer: &mut W) -> Result<(), std::io::Error> {
                        let version_number = 0;
                        writer.write_varint(version_number)?;

                        let kind_id = Into::<Entity>::into(self.clone()).kind().id();
                        writer.write_varint(kind_id)?;

                        Ok(match &self {
                            #(&EntityV0::#variants(ent) => serde_cbor::ser::to_writer_packed(writer, &ent.clone().to_compact_format()).unwrap()),
                            *
                        })
                    }

                    #[cfg(feature = "serialize2")]
                    pub fn serialize(&self, buf: &mut [u8]) -> Result<&[u8], ()> {
                        unsigned_varint::encode::u8(version_number, &mut buf[0..2]);
                        unsigned_varint::encode::u8(version_number, &mut buf[2..4]);

                        // let kind_id = Into::<Entity>::into(self.clone()).kind().id();
                        // writer.write_varint(kind_id)?;

                        // TODO
                        unimplemented!()
                    }

                    #[cfg(feature = "std")]
                    pub fn deserialize<R: ::std::io::Read>(reader: &mut R) -> Result<Self, std::io::Error> {
                        let version_number: u64 = reader.read_varint()?;
                        if version_number != 0 {
                            // TODO
                            panic!("Can only parse version 0 entity.");
                        }

                        let kind_id: u64 = reader.read_varint()?;
                        Ok(match kind_id {
                            #(#kind_ids => EntityV0::#variants2(FormatCompact::from_compact_format(serde_cbor::de::from_reader(reader).unwrap()))),
                            *,
                            // TODO
                            _ => panic!("Unrecognized kind id.")
                        })
                    }
                }
            };
            write!(writer, "{}", trait_impl).unwrap();
        }
    }
}
