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
    let iter_struct_name: syn::Type = syn::parse_str(&format!("{}CidFields", kind_name)).unwrap();
    let iter_struct: TokenStream = parse_quote! {
        pub struct #iter_struct_name<'a> {
            inner: &'a #kind_ty,
            field_index: usize,
            #[allow(dead_code)]
            field_vec_index: usize,
        }
    };
    write!(writer, "{}", iter_struct).unwrap();

    let kind_ty: syn::Type = syn::parse_str(kind_name).unwrap();
    let iter_struct_name: syn::Type = syn::parse_str(&format!("{}CidFields", kind_name)).unwrap();
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

    let iter_struct_name: syn::Type = syn::parse_str(&format!("{}CidFields", kind_name)).unwrap();
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
    let iter_struct_name: syn::Type = syn::parse_str(&format!("{}CidFields", kind_name)).unwrap();
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
        let type_impl: TokenStream = parse_quote! {
            #[derive(Debug, Clone, PartialEq, strum_macros::EnumVariantNames)]
            pub enum EntityKind {
                #(#variants),
                *
            }
        };
        write!(writer, "{}", type_impl).unwrap();
    }
    // Into<&'a str>
    {
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
                        #(EntityKind::#variants => #variants::default().into()),*
                    }
                }

                pub fn id(&self) -> u64 {
                    match self {
                        #(EntityKind::#variants => #kind_ids),*
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
        let type_impl: TokenStream = parse_quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub enum Entity {
                #(#variants(#variants)),
                *
            }
        };
        write!(writer, "{}", type_impl).unwrap();
    }
    // impl ToCid
    {
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
        let variants_iter_structs: Vec<syn::Type> = kind_names
            .clone()
            .into_iter()
            .map(|variant| syn::parse_str(&format!("{}CidFields", variant)).unwrap())
            .collect();

        let enum_impl: TokenStream = parse_quote! {
            pub enum EntityCidFields<'a> {
                #(#variants(#variants_iter_structs<'a>)),
                *
            }
        };
        write!(writer, "{}", enum_impl).unwrap();

        let enum_impl_iterator: TokenStream = parse_quote! {
            impl<'a> Iterator for EntityCidFields<'a> {
                type Item = &'a Vec<u8>;

                fn next(&mut self) -> Option<Self::Item> {
                    match self {
                        #(EntityCidFields::#variants(inner) => inner.next()),
                        *
                    }
                }
            }
        };
        write!(writer, "{}", enum_impl_iterator).unwrap();

        let trait_impl: TokenStream = parse_quote! {
            impl<'a> CidFields<'a> for Entity {
                type Iter = EntityCidFields<'a>;

                fn iter_cid_fields(&'a self) -> EntityCidFields {
                    match self {
                        #(Entity::#variants(inner) => EntityCidFields::#variants(inner.iter_cid_fields())),
                        *
                    }
                }
            }
        };
        write!(writer, "{}", trait_impl).unwrap();
    }
    // impl Entity
    {
        let type_impl: TokenStream = parse_quote! {
            impl Entity {
                pub fn kind(&self) -> EntityKind {
                    match &self {
                        #(Entity::#variants(_) => EntityKind::#variants),
                        *
                    }
                }
            }
        };
        write!(writer, "{}", type_impl).unwrap();
    }
    // impl FromABIV2ResponseHinted
    {
        let trait_impl: TokenStream = parse_quote! {
            #[cfg(feature = "web3_compat")]
            impl FromABIV2ResponseHinted for Entity {
                fn from_abiv2(bytes: &[u8], kind: &EntityKind) -> Self {
                    match kind {
                        #(EntityKind::#variants => Entity::#variants(FromABIV2Response::from_abiv2(bytes))),
                        *
                    }
                }
            }
        };
        write!(writer, "{}", trait_impl).unwrap();
    }
}