module.exports = {
  parsed: [
    {
      lhs: 'Axiom',
      rhs: [
        // 'Declaration',
        'ClassAxiom',
        // "ObjectPropertyAxiom",
        // "DataPropertyAxiom",
        // "DatatypeDefinition",
        // "HasKey",
        // "Assertion",
        // "AnnotationAxiom"
      ],
    },
    {
      lhs: 'ClassAxiom',
      rhs: [
        'SubClassOf',
        // "EquivalentClasses",
        // "DisjointClasses",
        // "DisjointUnion"
      ],
    },
    {
      lhs: 'SubClassOf',
      rhs: {
        type: 'function',
        name: 'SubClassOf',
        params: [
          'axiomAnnotations',
          'subClassExpression',
          'superClassExpression',
        ],
      },
    },
    {
      lhs: 'Declaration',
      rhs: {
        type: 'function',
        name: 'Declaration',
        params: ['axiomAnnotations', 'Entity'],
      },
    },
    {
      lhs: 'subClassExpression',
      rhs: ['ClassExpression'],
    },
    {
      lhs: 'superClassExpression',
      rhs: ['ClassExpression'],
    },
    {
      lhs: 'ClassExpression',
      rhs: [
        'Class',
        // "ObjectIntersectionOf",
        // "ObjectUnionOf",
        'ObjectComplementOf',
        // "ObjectOneOf",
        // "ObjectSomeValuesFrom",
        // "ObjectAllValuesFrom",
        // "ObjectHasValue",
        // "ObjectHasSelf",
        // "ObjectMinCardinality",
        // "ObjectMaxCardinality",
        // "ObjectExactCardinality",
        // "DataSomeValuesFrom",
        // "DataAllValuesFrom",
        // "DataHasValue",
        // "DataMinCardinality",
        // "DataMaxCardinality",
        // "DataExactCardinality"
      ],
    },
    {
      lhs: 'axiomAnnotations',
      rhs: {
        type: 'zeroOrMore',
        data: 'Annotation',
      },
    },
    {
      lhs: 'Entity',
      rhs: [
        'Class',
        // "Datatype",
        // "ObjectProperty",
        // "DataProperty",
        'AnnotationProperty',
        // "NamedIndividual"
      ],
    },
    { lhs: 'Class', rhs: ['IRI'] },
    // { lhs: 'Datatype', rhs: ['IRI'] },
    { lhs: 'AnnotationProperty', rhs: ['IRI'] },
    {
      lhs: 'Annotation',
      rhs: {
        type: 'function',
        name: 'Annotation',
        params: [
          'annotationAnnotations',
          'AnnotationProperty',
          'AnnotationValue',
        ],
      },
    },
    {
      lhs: 'annotationAnnotations',
      rhs: {
        type: 'zeroOrMore',
        data: 'Annotation',
      },
    },
    {
      lhs: 'AnnotationValue',
      rhs: [
        // "AnonymousIndividual",
        'IRI',
        // "Literal"
      ],
    },
    {
      lhs: 'ObjectComplementOf',
      rhs: {
        type: 'function',
        name: 'ObjectComplementOf',
        params: ['ClassExpression'],
      },
    },
  ],
};
