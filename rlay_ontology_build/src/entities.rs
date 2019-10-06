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
