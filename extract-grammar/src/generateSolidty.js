const fs = require('fs');

const grammar = require('../build/intermediate.json');
const addOntologyStorageContract = require('./generateSolidityContracts.js')
  .addOntologyStorageContract;
const addOntologyStorageLibraryContract = require('./generateSolidityContracts.js')
  .addOntologyStorageLibraryContract;
const addKindStorageInterfaceContracts = require('./generateSolidityContracts.js')
  .addKindStorageInterfaceContracts;
const addKindStorageContracts = require('./generateSolidityContracts.js')
  .addKindStorageContracts;

const indent = i => Array(i + 1).join(' ');
const indentLevel = i => Array(i * 4 + 1).join(' ');

const addHeader = protobuf => {
  protobuf.file += 'pragma solidity ^0.4.21;\n';
  protobuf.file += 'pragma experimental ABIEncoderV2;\n';
  protobuf.file += '\n';
  protobuf.file += 'import "./cid.sol";\n';
  protobuf.file += 'import "./pb_mod.sol";\n';
  protobuf.file += '\n';
};

const main = () => {
  const solidity = { file: '' };

  addHeader(solidity);
  addKindStorageInterfaceContracts(solidity, grammar);
  addKindStorageContracts(solidity, grammar);
  addOntologyStorageContract(solidity, grammar);

  const solidityFile = solidity.file;
  console.log(solidityFile);
  fs.writeFile('build/OntologyStorage.sol', solidityFile, function(err) {
    if (err) throw err;
  });
};

main();
