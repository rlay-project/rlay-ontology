Related to: https://www.w3.org/TR/owl2-syntax

- Declarations as specified in the document are superflous, as we base out concept around content-addressing
- Instead of declarations entities are defined by their EnityAxiom (e.g. ClassAxiom)
  - Regarding the EntityAxiom care needs to be taken regarding immutability properties of the individual variants (e.g. EquivalentClasses can mutate meaning of existing ontology)
  - Annotations are then also folded into the declarations via Axioms
