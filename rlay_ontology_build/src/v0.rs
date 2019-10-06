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
        let type_impl: TokenStream = parse_quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub enum EntityV0 {
                #(#variants(#variants)),
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
        let type_impl: TokenStream = parse_quote! {
            impl Into<Entity> for EntityV0 {
                fn into(self) -> Entity {
                    match self {
                        #(EntityV0::#variants(ent) => Entity::#variants(ent)),
                        *
                    }
                }
            }
        };
        write!(writer, "{}", type_impl).unwrap();
    }
    // impl Into<EntityV0>
    {
        let type_impl: TokenStream = parse_quote! {
            impl Into<EntityV0> for Entity {
                fn into(self) -> EntityV0 {
                    match self {
                        #(Entity::#variants(ent) => EntityV0::#variants(ent)),
                        *
                    }
                }
            }
        };
        write!(writer, "{}", type_impl).unwrap();
    }
    // impl EntityV0
    {
        let trait_impl: TokenStream = parse_quote! {
            impl EntityV0 {
                #[cfg(feature = "std")]
                pub fn serialize<W: ::std::io::Write>(&self, writer: &mut W) -> Result<(), std::io::Error> {
                    let version_number = 0;
                    writer.write_varint(version_number)?;

                    let kind_id = Into::<Entity>::into(self.clone()).kind().id();
                    writer.write_varint(kind_id)?;

                    Ok(match &self {
                        #(&EntityV0::#variants(ent) => serde_cbor::ser::to_writer(writer, &ent.clone().to_compact_format()).unwrap()),
                        *
                    })
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
                        #(#kind_ids => EntityV0::#variants(FormatCompact::from_compact_format(serde_cbor::de::from_reader(reader).unwrap()))),
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
