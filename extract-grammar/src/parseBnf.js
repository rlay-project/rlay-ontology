const peg = require('pegjs');
const fs = require('fs');
const path = require('path');

const parseGrammar = grammar => {
  const parserGrammar = fs.readFileSync(
    path.join(__dirname, './bnf_grammar.pegjs'),
    'utf8'
  );
  const parser = peg.generate(parserGrammar);

  return parser.parse(grammar);
};

module.exports = {
  parseGrammar,
};
