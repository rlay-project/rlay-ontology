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
            let offset_ident = format_ident!("{}_offset", field.name.to_snake_case());
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
            let offset_ident = format_ident!("{}_offset", field.name.to_snake_case());

            let next_field = fields.get(i + 1);
            let tokens: TokenStream = match next_field {
                Some(next_field) => {
                    let next_offset_ident = format_ident!("{}_offset", next_field.name.to_snake_case());
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
                                #(Entity::#variants(ent) => EntityFormatWeb3::#variants(ent.into())),
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
                                #(EntityFormatWeb3::#variants(ent) => Entity::#variants(ent.0)),
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
    let field_idents: Vec<syn::Ident> = fields.iter().map(|field| field.field_ident()).collect();
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
                            return Err(de::Error::duplicate_field(#field_names_raw));
                        }}
                        #extract_key_blocks
                    }
                 )*
                // ignore unknown fields
                Some(_) => {}
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
