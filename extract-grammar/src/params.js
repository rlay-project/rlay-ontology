const assert = require('assert');
const mapperHelpers = require('./mapperHelpers');

module.exports = {
  mappers: {
    Annotation: {
      functionParams: params => {
        assert(params.length === 3);
        return [
          mapperHelpers.annotationField(params[0]),
          {
            name: 'property',
            kind: params[1],
            required: true,
          },
          {
            name: 'value',
            kind: params[2],
            required: true,
          },
        ];
      },
    },
    AnnotationProperty: {
      asFieldKind: rhs => rhs,
    },
    AnnotationValue: {
      asFieldKind: rhs => {
        assert(rhs.length === 3);
        assert(rhs[1] === 'IRI');
        // TODO: find a better way for "IRI or Literal"
        return rhs[1];
      },
    },
    annotationAnnotations: {
      asFieldKind: rhs => {
        assert(rhs.type === 'zeroOrMore');
        return `${rhs.name}[]`;
      },
    },
    axiomAnnotations: {
      asFieldKind: rhs => {
        assert(rhs.type === 'zeroOrMore');
        return `${rhs.name}[]`;
      },
    },
    ObjectComplementOf: {
      functionParams: params => {
        assert(params.length === 1);
        return [
          {
            name: 'complementOf',
            kind: params[0],
            required: true,
          },
        ];
      },
    },
    SubClassOf: {
      checkExpressionKind: expressionKind => {
        return expressionKind === 'ClassExpression';
      },
      functionParams: params => {
        assert(params.length === 3);
        return [
          {
            name: 'annotations',
            kind: params[0],
          },
          // skip subClassExpression
          {
            name: 'superClassExpression',
            kind: params[2],
          },
        ];
      },
    },
    // DisjointClasses: {
      // checkExpressionKind: expressionKind => {
        // return expressionKind === 'ClassExpression';
      // },
      // functionParams: params => {
        // assert(params.length === 4);
        // return [
          // {
            // name: 'annotations',
            // kind: params[0],
          // },
          // {
            // name: 'disjointClasses',
            // kind: `${params[1]}[]`,
          // },
        // ];
      // },
    // },
    SubObjectPropertyOf: {
      checkExpressionKind: expressionKind => {
        return expressionKind === 'ObjectPropertyExpression';
      },
      functionParams: params => {
        assert(params.length === 3);
        return [
          {
            name: 'annotations',
            kind: params[0],
          },
          // skip subObjectPropertyExpression
          {
            name: 'superObjectPropertyExpression',
            kind: params[2],
          },
        ];
      },
    },
    SubDataPropertyOf: {
      checkExpressionKind: expressionKind => {
        return expressionKind === 'DataPropertyExpression';
      },
      functionParams: params => {
        assert(params.length === 3);
        return [
          {
            name: 'annotations',
            kind: params[0],
          },
          // skip subDataPropertyExpression
          {
            name: 'superDataPropertyExpression',
            kind: params[2],
          },
        ];
      },
    },
  },
  restrictGrammar: {
    removedDeclarations: [
      'EquivalentClasses',
    ],
    Axiom: definition => {
      definition.rhs = definition.rhs.filter(n => n !== 'Declaration');
      // TODO: not parsable from grammar yet or can't be processed
      definition.rhs = definition.rhs.filter(n => n !== 'DatatypeDefinition');
      definition.rhs = definition.rhs.filter(n => n !== 'HasKey');
      return definition;
    },
    // TODO
    ClassExpression: definition => {
      // definition.rhs = ['Class', 'ObjectComplementOf'];
      return definition;
    },
  },
};
