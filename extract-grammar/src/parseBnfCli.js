const fs = require('fs');
const path = require('path');

const parser = require('./parseBnf');

const main = () => {
  const grammar = fs.readFileSync(
    path.join(__dirname, './__tests__/implemented.grammar'),
    'utf8'
  );

  const parsed = parser.parseGrammar(grammar);
  const grammarContents = {
    parsed,
  };
  fs.writeFile(
    'build/grammar.json',
    JSON.stringify(grammarContents, null, 2),
    function(err) {
      if (err) throw err;
    }
  );
};

main();
