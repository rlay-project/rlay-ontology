const fs = require('fs');

const grammar = require('../build/intermediate.json');

const addHeader = protobuf => {
  protobuf.file += 'syntax = "proto2";\n';
  protobuf.file += '\n';
  protobuf.file += 'package rlay.ontology;\n';
  protobuf.file += '\n';
};

const addMessages = (protobuf, grammar) => {
  const { kinds } = grammar;
  kinds.forEach(kind => {
    // message header
    protobuf.file += `message ${kind.name} {\n`;

    kind.fields.forEach((field, i) => {
      // padding
      protobuf.file += '  ';
      if (field.kind.endsWith('[]')) {
        protobuf.file += `repeated`;
      } else if (field.required) {
        protobuf.file += `required`;
      } else {
        protobuf.file += `optional`;
      }
      protobuf.file += ` bytes ${field.name} = ${i + 1};\n`;
    });

    // message closing
    protobuf.file += '}\n';
    protobuf.file += '\n';
  });
};

const main = () => {
  const protobuf = { file: '' };

  addHeader(protobuf);
  addMessages(protobuf, grammar);

  const protobufFile = protobuf.file;
  console.log(protobufFile);
  fs.writeFile('build/ontology_pb2.proto', protobufFile, function(err) {
    if (err) throw err;
  });
};

main();
