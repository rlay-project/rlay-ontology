use rlay_ontology;
use rlay_ontology::ToCidUnknown;
use rlay_ontology::ontology::{Annotation, AssociatedCodec, Class, Individual as OntologyIndividual};
use multihash::{decode as mh_decode, encode as mh_encode, Hash};
use multibase::decode as mb_decode;
use cid::ToCid;

#[derive(Serialize, Deserialize, Debug)]
pub struct SerializableAnnotation {
    /// IRI
    pub property: String,
    /// IRI / Literal
    pub value: String,
}

impl From<SerializableAnnotation> for Annotation {
    fn from(val: SerializableAnnotation) -> Annotation {
        let property = match mb_decode(&val.property) {
            Ok((_, property_bytes)) => match mh_decode(&property_bytes) {
                Ok(_) => property_bytes,
                Err(_) => unimplemented!(),
            },
            Err(_) => {
                warn!("Value {:?} can not be interpreted as the hash of an AnnotationProperty. Treating it as raw IRI and hashing it.", &val.property);
                let hashed_property = mh_encode(Hash::SHA3256, val.property.as_bytes()).unwrap();
                hashed_property
            }
        };

        Annotation {
            property: property,
            value: val.value,
        }
    }
}

/// Node without probability function
#[derive(Serialize, Deserialize, Debug)]
#[deprecated(note = "Please use SerializableClassV2 instead")]
pub struct SerializableClass {
    pub label: String,
    pub parents: Vec<String>,
}

impl SerializableClass {
    pub fn new<S1: Into<String>, S2: Into<String>>(raw_label: S1, raw_parents: Vec<S2>) -> Self {
        let parents: Vec<_> = raw_parents.into_iter().map(|n| n.into()).collect();

        Self {
            label: raw_label.into(),
            parents,
        }
    }
}

impl From<SerializableClass> for Class {
    fn from(val: SerializableClass) -> Class {
        let annotations = vec![
            val.label
                .to_cid_unknown(<Annotation as AssociatedCodec>::CODEC_CODE)
                .unwrap()
                .to_bytes(),
        ];
        let sub_class_of_class = val.parents
            .iter()
            .map(|n| {
                n.to_cid_unknown(<Class as AssociatedCodec>::CODEC_CODE)
                    .unwrap()
                    .to_bytes()
            })
            .collect();

        Class {
            annotations,
            sub_class_of_class,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SerializableClassV2 {
    pub annotations: Vec<String>,
    pub sub_class_of_class: Vec<String>,
}

impl From<SerializableClassV2> for Class {
    fn from(val: SerializableClassV2) -> Class {
        let annotations = val.annotations
            .iter()
            .map(|n| {
                n.to_cid_unknown(<Annotation as AssociatedCodec>::CODEC_CODE)
                    .unwrap()
                    .to_bytes()
            })
            .collect();
        let sub_class_of_class = val.sub_class_of_class
            .iter()
            .map(|n| {
                n.to_cid_unknown(<Class as AssociatedCodec>::CODEC_CODE)
                    .unwrap()
                    .to_bytes()
            })
            .collect();

        Class {
            annotations,
            sub_class_of_class,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SerializableIndividualV2 {
    pub annotations: Vec<String>,
    pub class_assertions: Vec<String>,
    pub negative_class_assertions: Vec<String>,
}

impl From<SerializableIndividualV2> for OntologyIndividual {
    fn from(val: SerializableIndividualV2) -> OntologyIndividual {
        let annotations = val.annotations
            .iter()
            .map(|n| {
                n.to_cid_unknown(<Annotation as AssociatedCodec>::CODEC_CODE)
                    .unwrap()
                    .to_bytes()
            })
            .collect();
        let class_assertions = val.class_assertions
            .iter()
            .map(|n| {
                n.to_cid_unknown(<Class as AssociatedCodec>::CODEC_CODE)
                    .unwrap()
                    .to_bytes()
            })
            .collect();
        let negative_class_assertions = val.negative_class_assertions
            .iter()
            .map(|n| {
                n.to_cid_unknown(<Class as AssociatedCodec>::CODEC_CODE)
                    .unwrap()
                    .to_bytes()
            })
            .collect();

        OntologyIndividual {
            annotations,
            class_assertions,
            negative_class_assertions,
        }
    }
}

#[derive(Serialize)]
pub struct SerializableVariable {
    variable: String,
    value: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[deprecated(note = "Please use SerializableIndividualV2 instead")]
pub struct Individual {
    pub label: String,
    pub class_memberships: Vec<String>,
}

impl Individual {
    pub fn new<S1: Into<String>, S2: Into<String>>(
        raw_label: S1,
        raw_class_memberships: Vec<S2>,
    ) -> Self {
        let class_memberships: Vec<_> = raw_class_memberships
            .into_iter()
            .map(|n| n.into())
            .collect();

        Self {
            label: raw_label.into(),
            class_memberships,
        }
    }
}

js_serializable!(SerializableClass);
js_deserializable!(SerializableClass);
js_serializable!(SerializableClassV2);
js_deserializable!(SerializableClassV2);

js_serializable!(SerializableAnnotation);
js_deserializable!(SerializableAnnotation);

js_serializable!(Individual);
js_deserializable!(Individual);
js_serializable!(SerializableIndividualV2);
js_deserializable!(SerializableIndividualV2);
