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
    let constructor_call: TokenStream = parse_quote! {
        Ok(#wrapper_ty {
            inner: #kind_ty {
                #(#field_idents: helper_instance.#field_idents),
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

                #[allow(dead_code)]
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
