macro_rules! impl_to_cid {
        ($v:path) => (
            impl ToCid for $v {
                fn to_cid(&self) -> Result<Cid, CidError> {
                    let mut encoded = Vec::<u8>::new();
                    self.encode(&mut encoded).map_err(|_| CidError::ParsingError)?;
                    let hashed = encode(Hash::Keccak256, &encoded).map_err(|_| CidError::ParsingError)?;

                    let cid = Cid::new(Codec::Unknown(<Self as AssociatedCodec>::CODEC_CODE), Version::V1, &hashed);
                    Ok(cid)
                }
            })
        ;
    }

macro_rules! codec_code {
        ($v:path, $c:expr) => (
            impl AssociatedCodec for $v {
                const CODEC_CODE: u64 = $c;
            }
        );
    }

macro_rules! impl_canonicalize {
        ($v:path; $($field_name:ident),*) => (
            impl Canonicalize for $v {
                fn canonicalize(&mut self) {
                    $(self.$field_name.sort());*
                }
            }
        );
    }

macro_rules! impl_into_entity_kind {
        ($v:path, $wrapper:path) => (
            impl Into<Entity> for $v {
                fn into(self) -> Entity {
                    $wrapper(self)
                }
            }
        );
    }
