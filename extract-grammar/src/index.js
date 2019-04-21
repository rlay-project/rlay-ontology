const fs = require('fs');
const varint = require('varint');

const parsed = require('../build/grammar.json').parsed;
const paramsConfig = require('./params.js');

const checkDependencies = parsedGrammar => {
  const lhsIdentifiers = parsedGrammar.map(definition => definition.lhs);

  const checkRHSArray = alternatives => {
    alternatives.forEach(alternative => {
      const exists = lhsIdentifiers.includes(alternative);
      if (!exists) {
        console.warn('RHS alternative', alternative, 'does not exist');
      }
    });
  };

  parsedGrammar.forEach(definition => {
    if (Array.isArray(definition.rhs)) {
      checkRHSArray(definition.rhs);
    } else if (definition.rhs.type === 'function') {
      checkRHSArray(definition.rhs.params);
    } else if (definition.rhs.type === 'zeroOrMore') {
      checkRHSArray([definition.rhs.data]);
    } else {
      console.warn(`Checking of RHS for ${definition.lhs} not implemented yet`);
    }
  });
};

const kindFieldsFromFunction = funcExpression => {
  const mappers = paramsConfig.mappers;
  let mapper = mappers[funcExpression.name];
  if (!mapper) {
    console.error(`No function param mapper found for ${funcExpression.name}`);
    return [];
  }
  mapper = mapper.functionParams;
  if (!mapper) {
    console.error(`No function param mapper found for ${funcExpression.name}`);
    return [];
  }

  return mapper(funcExpression.params);
};

const kindFieldsFromAxiom = (kind, axiom) => {
  let isAxiomApplicable = true;

  const mappers = paramsConfig.mappers;
  const mapper = mappers[axiom.rhs.name];
  if (!mapper) {
    isAxiomApplicable = true;
  } else {
    const expressionCheck = mapper.checkExpressionKind;
    if (!expressionCheck) {
      isAxiomApplicable = true;
    } else {
      isAxiomApplicable = expressionCheck(kind.expressionKind);
    }
  }

  if (!isAxiomApplicable) {
    return [];
  }
  return kindFieldsFromFunction(axiom.rhs);
};

const buildExpression = expression => {
  const kind = {
    name: expression.lhs,
    fields: [],
  };

  if (
    expression.rhs === 'IRI' ||
    (Array.isArray(expression.rhs) && expression.rhs[0] === 'IRI')
  ) {
    // Base declarations don't get a IRI
  } else if (expression.rhs.type === 'function') {
    kind.fields = kind.fields.concat(kindFieldsFromFunction(expression.rhs));
  } else {
    console.log(
      `Not implemented yet - buildExpression for ${JSON.stringify(expression)}`
    );
  }

  return kind;
};

const decorateKindWithAxiom = (kind, axiom) => {
  kind.fields = kind.fields.concat(kindFieldsFromAxiom(kind, axiom));
};

const restrictGrammar = parsedGammar => {
  const restrictDefinition = definition => {
    const mappers = paramsConfig.restrictGrammar;
    const mapper = mappers[definition.lhs];
    if (!mapper) {
      return definition;
    }
    return mapper(definition);
  };

  return parsedGammar.map(restrictDefinition);
};

// Build all kinds for expressions in one group, e.g. 'ClassExpression'
const buildExpressionsForGroup = (grammar, groupName) => {
  const groupExpressionKinds = [];
  const expressionsInGroup = grammar.find(n => n.lhs === groupName);

  const rhs = Array.isArray(expressionsInGroup.rhs) ? expressionsInGroup.rhs : [expressionsInGroup.rhs];
  rhs.forEach(expressionIdent => {
    const expression = grammar.find(n => n.lhs === expressionIdent);
    const kind = buildExpression(expression);
    kind.expressionKind = groupName;

    groupExpressionKinds.push(kind);
  });

  const axioms = grammar.find(n => n.lhs === 'Axiom');
  groupExpressionKinds.forEach(kind => {
    axioms.rhs.forEach(axiomGroupIdent => {
      const axiomGroup = grammar.find(n => n.lhs === axiomGroupIdent);
      axiomGroup.rhs.forEach(axiomIdent => {
        const axiom = grammar.find(n => n.lhs === axiomIdent);

        decorateKindWithAxiom(kind, axiom);
      });
    });
  });

  return groupExpressionKinds;
};

const buildGrammar = parsedGrammar => {
  let kinds = [];

  const grammar = restrictGrammar(parsedGrammar);

  const classExpressionKinds = buildExpressionsForGroup(
    grammar,
    'ClassExpression'
  );
  const objectPropertyExpressionKinds = buildExpressionsForGroup(
  grammar, 'ObjectPropertyExpression'
  );
  const dataPropertyExpressionKinds = buildExpressionsForGroup(
  grammar, 'DataPropertyExpression'
  );
  // Though they are named AnnotationAxiom, they are more like expressions
  const annotationAxiomKinds = buildExpressionsForGroup(
    grammar,
    'AnnotationAxiom'
  );

  const otherKinds = [];
  const annotationDeclaration = grammar.find(n => n.lhs === 'Annotation');
  otherKinds.push(buildExpression(annotationDeclaration));
  // TODO: proper
  otherKinds.push({
    name: 'Individual',
    fields: [
      {
        name: 'annotations',
        kind: 'Annotation[]',
      },
      {
        name: 'class_assertions',
        kind: 'ClassAssertion[]',
      },
      {
        name: 'negative_class_assertions',
        kind: 'NegativeClassAssertion[]',
      },
      {
        name: 'object_property_assertions',
        kind: 'ObjectPropertyAssertion[]',
      },
      {
        name: 'negative_object_property_assertions',
        kind: 'NegativeObjectPropertyAssertion[]',
      },
      {
        name: 'data_property_assertions',
        kind: 'DataPropertyAssertion[]',
      },
      {
        name: 'negative_data_property_assertions',
        kind: 'NegativeDataPropertyAssertion[]',
      },
    ],
  });
  otherKinds.push({
    name: 'AnnotationProperty',
    fields: [
      {
        name: 'annotations',
        kind: 'Annotation[]',
      },
    ],
  });
  otherKinds.push({
    name: 'ClassAssertion',
    fields: [
      {
        name: 'annotations',
        kind: 'Annotation[]',
      },
      {
        name: 'subject',
        kind: 'IRI',
      },
      {
        name: 'class',
        kind: 'IRI',
        required: true,
      },
    ],
  });
  otherKinds.push({
    name: 'NegativeClassAssertion',
    fields: [
      {
        name: 'annotations',
        kind: 'Annotation[]',
      },
      {
        name: 'subject',
        kind: 'IRI',
      },
      {
        name: 'class',
        kind: 'IRI',
        required: true,
      },
    ],
  });
  otherKinds.push({
    name: 'ObjectPropertyAssertion',
    fields: [
      {
        name: 'annotations',
        kind: 'Annotation[]',
      },
      {
        name: 'subject',
        kind: 'IRI',
      },
      {
        name: 'property',
        kind: 'IRI',
      },
      {
        name: 'target',
        kind: 'IRI',
      },
    ],
  });
  otherKinds.push({
    name: 'NegativeObjectPropertyAssertion',
    fields: [
      {
        name: 'annotations',
        kind: 'Annotation[]',
      },
      {
        name: 'subject',
        kind: 'IRI',
      },
      {
        name: 'property',
        kind: 'IRI',
      },
      {
        name: 'target',
        kind: 'IRI',
      },
    ],
  });
  otherKinds.push({
    name: 'DataPropertyAssertion',
    fields: [
      {
        name: 'annotations',
        kind: 'Annotation[]',
      },
      {
        name: 'subject',
        kind: 'IRI',
      },
      {
        name: 'property',
        kind: 'IRI',
      },
      {
        name: 'target',
        kind: 'IRI',
      },
    ],
  });
  otherKinds.push({
    name: 'NegativeDataPropertyAssertion',
    fields: [
      {
        name: 'annotations',
        kind: 'Annotation[]',
      },
      {
        name: 'subject',
        kind: 'IRI',
      },
      {
        name: 'property',
        kind: 'IRI',
      },
      {
        name: 'target',
        kind: 'IRI',
      },
    ],
  });
  otherKinds.push({
    name: 'AnnotationAssertion',
    fields: [
      {
        name: 'annotations',
        kind: 'Annotation[]',
      },
      {
        name: 'subject',
        kind: 'IRI',
      },
      {
        name: 'property',
        kind: 'IRI',
      },
      {
        name: 'value',
        kind: 'IRI',
      },
    ],
  });
  otherKinds.push({
    name: 'NegativeAnnotationAssertion',
    fields: [
      {
        name: 'annotations',
        kind: 'Annotation[]',
      },
      {
        name: 'subject',
        kind: 'IRI',
      },
      {
        name: 'property',
        kind: 'IRI',
      },
      {
        name: 'value',
        kind: 'IRI',
      },
    ],
  });
  otherKinds.push({
    name: 'Literal',
    fields: [
      {
        name: 'datatype',
        kind: 'IRI',
      },
      {
        name: 'value',
        kind: 'IRI',
        required: true,
      }
    ]
  });
  otherKinds.push({
    name: 'Datatype',
    fields: [
      {
        name: 'annotations',
        kind: 'Annotation[]',
      },
    ]
  });
  otherKinds.push({
    name: 'DataIntersectionOf',
    fields: [
      {
        name: 'annotations',
        kind: 'Annotation[]',
      },
      {
        name: 'datatypes',
        kind: 'DatatypeExpression[]',
      }
    ]
  });
  otherKinds.push({
    name: 'DataUnionOf',
    fields: [
      {
        name: 'annotations',
        kind: 'Annotation[]',
      },
      {
        name: 'datatypes',
        kind: 'DatatypeExpression[]',
      }
    ]
  });
  otherKinds.push({
    name: 'DataComplementOf',
    fields: [
      {
        name: 'annotations',
        kind: 'Annotation[]',
      },
      {
        name: 'datatype',
        kind: 'IRI', // DatatypeExpression
      }
    ]
  });
  otherKinds.push({
    name: 'DataOneOf',
    fields: [
      {
        name: 'annotations',
        kind: 'Annotation[]',
      },
      {
        name: 'values',
        kind: 'Literal[]',
      }
    ]
  });

  kinds = kinds.concat(
    classExpressionKinds,
    objectPropertyExpressionKinds,
    dataPropertyExpressionKinds,
    // annotationAxiomKinds,
    otherKinds
  );
  kinds.forEach(kind => kind.fields.sort((a, b) => a.name >= b.name));

  kinds.forEach(kind => {
    kind.fields.forEach(field => transformKindField(grammar, field));
  });

  kinds = kinds.map((kind, i) => Object.assign({}, kind, {
    kindId: i,
    cidPrefix: calculateCidPrefix(i),
    cidPrefixHex: calculateCidPrefixHex(i),
    fields: uniqFields(kind.fields),
  }));

  return {
    kinds,
  };
};

const calculateCidPrefix = (i) => {
  const base = 0xc000;
  const cidPrefixNumber = base + i;

  return cidPrefixNumber;
}

const calculateCidPrefixHex = (i) => {
  const cidPrefixNumber = calculateCidPrefix(i);
  const bytes = Buffer.from(varint.encode(cidPrefixNumber));
  return bytes.toString('hex');
}

const transformKindField = (grammar, field) => {
  const hackyFieldKind = hackyFieldKindTransformation(field);
  if (hackyFieldKind) {
    console.log('HACKY transform');
    field.kind = hackyFieldKind;
    return;
  }

  const fieldExpression = grammar.find(n => n.lhs === field.kind);
  const mappers = paramsConfig.mappers;
  let mapper = mappers[field.kind];
  if (!mapper) {
    console.error(`No asFieldKind mapper found for ${field.kind}`);
    return;
  }
  mapper = mapper.asFieldKind;
  if (!mapper) {
    console.error(`No asFieldKind mapper mapper found for ${field.kind}`);
    return [];
  }

  field.kind = mapper(fieldExpression.rhs);
};

// Unify field names that have same name and same kind
const uniqFields = (fields) => {
  const filteredFields = [];
  fields.forEach((field) => {
    if (!filteredFields.map(n => n.name).includes(field.name)) {
      filteredFields.push(field);
      return;
    }
    if (field.name === 'annotations') {
      // duplicate annotations fields are ignored as they are produced by a lot of entities
      return;
    }
    throw new Error('Unexpected duplicate field', field.name);
  });

  return filteredFields;
}

// TODO: move to params.js
// Transforms grammar identifiers of kind fields to other grammar identifiers (ending with "[]" if they are arrays)
const hackyFieldKindTransformation = field => {
  if (field.kind === 'superClassExpression') {
    return 'ClassExpression[]';
  }
  if (field.kind === 'superObjectPropertyExpression') {
    return 'ObjectProperyExpression[]';
  }
  if (field.kind === 'superDataPropertyExpression') {
    return 'DataPropertyExpression[]';
  }
  return null;
};

const main = () => {
  checkDependencies(parsed);
  const newGrammar = buildGrammar(parsed);

  console.log('New Grammar:');
  console.log(JSON.stringify(newGrammar.kinds, null, 2));
  fs.writeFile(
    'build/intermediate.json',
    JSON.stringify(newGrammar, null, 2),
    function(err) {
      if (err) throw err;
    }
  );
};

main();
