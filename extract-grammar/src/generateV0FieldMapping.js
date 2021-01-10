const fs = require('fs');

const grammar = require('../build/intermediate.json');

const main = () => {
  const fields = {};
  grammar.kinds.forEach(kind => {
    kind.fields.forEach(field => {
      if (!fields[field.name]) {
        fields[field.name] = 0;
      }
      fields[field.name] += 1;
    })
  });

  const fieldEntries = Array.from(Object.entries(fields));
  fieldEntries.sort((a, b) => b[1] - a[1]);

  let mapping = {};
  fieldEntries.forEach(([fieldName, count], i) => {
    mapping[i] = fieldName;
  });
  const mappingContents = JSON.stringify(mapping, null, 4);
  console.log(mappingContents);
  fs.writeFile('build/v0_field_mapping.json', mappingContents, function(err) {
    if (err) throw err;
  });
};

main();
