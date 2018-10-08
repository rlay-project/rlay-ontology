const fs = require('fs');
const path = require('path');

const parser = require('../parseBnf');

const readGrammar = filename => {
  return fs.readFileSync(path.join(__dirname, `${filename}.grammar`), 'utf8');
};

describe('parseBnf', () => {
  test('simple definition', () => {
    const grammar = readGrammar('simple');

    const parsed = parser.parseGrammar(grammar)[0];
    expect(parsed.lhs).toBe('LHS');
    expect(parsed.rhs).toBe('RHS');
  });

  test('multiple definitions', () => {
    const grammar = readGrammar('multiple_definitions');

    const parsed = parser.parseGrammar(grammar);
    expect(parsed.length).toBe(3);
  });

  test('RHS alternatives singleline', () => {
    const grammar = readGrammar('rhs_multiple_singleline');

    const parsed = parser.parseGrammar(grammar)[0];
    expect(parsed.lhs).toBe('ClassAxiom');
    expect(Array.isArray(parsed.rhs)).toBe(true);
    expect(parsed.rhs.length).toBe(4);
  });

  test('RHS alternatives multiline', () => {
    const grammar = readGrammar('rhs_multiple_multiline');

    const parsed = parser.parseGrammar(grammar)[0];
    expect(parsed.lhs).toBe('Assertion');
    expect(Array.isArray(parsed.rhs)).toBe(true);
  });

  test('RHS function with single param', () => {
    const grammar = readGrammar('rhs_function_single_param');

    const parsed = parser.parseGrammar(grammar)[0];
    expect(parsed.lhs).toBe('InverseObjectProperty');
    expect(parsed.rhs.type).toBe('function');
    expect(parsed.rhs.name).toBe('ObjectInverseOf');
    expect(parsed.rhs.params[0]).toBe('ObjectProperty');
  });

  test('RHS function with multiple params', () => {
    const grammar = readGrammar('rhs_function');

    const parsed = parser.parseGrammar(grammar)[0];
    expect(parsed.lhs).toBe('TransitiveObjectProperty');
    expect(parsed.rhs.type).toBe('function');
    expect(parsed.rhs.name).toBe('TransitiveObjectProperty');
    expect(parsed.rhs.params.length).toBe(2);
  });

  test('RHS function with mixed params', () => {
    const grammar = readGrammar('rhs_mixed_function');

    const parsed = parser.parseGrammar(grammar)[0];
    expect(parsed.lhs).toBe('DisjointDataProperties');
    expect(parsed.rhs.type).toBe('function');
    expect(parsed.rhs.name).toBe('DisjointDataProperties');
    expect(parsed.rhs.params.length).toBe(4);
  });

  test('RHS zero or more', () => {
    const grammar = readGrammar('rhs_zero_or_more');

    const parsed = parser.parseGrammar(grammar)[0];
    expect(parsed.lhs).toBe('axiomAnnotations');
    expect(parsed.rhs.type).toBe('zeroOrMore');
    expect(parsed.rhs.name).toBe('Annotation');
  });

  test('RHS zero or one', () => {
    const grammar = readGrammar('rhs_zero_or_one');

    const parsed = parser.parseGrammar(grammar)[0];
    expect(parsed.lhs).toBe('axiomAnnotations');
    expect(parsed.rhs.type).toBe('zeroOrOne');
    expect(parsed.rhs.name).toBe('Annotation');
  });

  describe('examples', () => {
    test('section_1', () => {
      const grammar = readGrammar('section_1');
      parser.parseGrammar(grammar);
    });

    test.skip('full', () => {
      const grammar = readGrammar('full');
      parser.parseGrammar(grammar);
    });

    test('implemented', () => {
      const grammar = readGrammar('implemented');
      parser.parseGrammar(grammar);
    });
  });
});
