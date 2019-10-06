#![allow(unused_imports)]
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

mod compact;
mod core;
mod entities;
mod intermediate;
mod v0;
mod web3;

use heck::SnakeCase;
use proc_macro2::TokenStream;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use crate::intermediate::{parse_intermediate_contents, Field, Kind};

pub fn build_files() {
    entities::build_file("src/intermediate.json", "rlay.ontology.entities.rs");
    fmt_file("rlay.ontology.entities.rs");
    core::build_macros_applied_file("src/intermediate.json", "rlay.ontology.macros_applied.rs");
    fmt_file("rlay.ontology.macros_applied.rs");
    web3::build_applied_file("src/intermediate.json", "rlay.ontology.web3_applied.rs");
    fmt_file("rlay.ontology.web3_applied.rs");
    compact::build_file("src/intermediate.json", "rlay.ontology.compact.rs");
    fmt_file("rlay.ontology.compact.rs");
    v0::build_file("src/intermediate.json", "rlay.ontology.v0.rs");
    fmt_file("rlay.ontology.v0.rs");
}

fn fmt_file(path: &str) {
    let rustfmt_available = Command::new("which")
        .arg("rustfmt")
        .output()
        .unwrap()
        .status
        .success();
    if !rustfmt_available {
        return;
    }

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join(path);
    Command::new("rustfmt").arg(dest_path).output().unwrap();
}

fn kind_names_types(kind_names: &[String]) -> Vec<syn::Type> {
    kind_names
        .iter()
        .map(|n| syn::parse_str(n).unwrap())
        .collect()
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
        let to_fn_ident = format_ident!("to_{}_format", format_suffix_lc);
        let from_fn_ident = format_ident!("from_{}_format", format_suffix_lc);

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
