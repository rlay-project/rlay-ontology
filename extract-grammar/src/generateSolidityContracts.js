const libName = 'OntologyStorageLib';
const indentLevel = i => Array(i * 4 + 1).join(' ');

const storageMapName = kind => {
  return `${kind.name.toLowerCase()}_hash_map`;
};

const storageDelegateName = kind => {
  return `${kind.name.toLowerCase()}_storage`;
};

const codecName = kind => {
  return `${kind.name}Codec`;
};

const codecClassName = kind => {
  return `${codecName(kind)}.${kind.name}`;
};

// returns a rendered string of kind params for use in function signatures
const kindParamsWithType = kind => {
  return kind.fields
    .map(field => `${solidityTypeForParamKind(field.kind)} _${field.name}`)
    .join(', ');
};

const kindParams = kind => {
  return kind.fields.map(field => `_${field.name}`).join(', ');
};

const solidityTypeForParamKind = paramKind => {
  if (paramKind.endsWith('[]')) {
    return 'bytes[]';
  } else {
    return 'bytes';
  }
};

const paramsToInstance = kind => {
  let line = '';

  line += indentLevel(2);
  line += `${codecClassName(kind)} memory _instance = ${codecClassName(kind)}(`;
  line += kindParams(kind);
  line += ');\n';

  return line;
};

const addCidConstant = (protobuf, kind) => {
  // TODO: per-kind value
  protobuf.file += `${indentLevel(1)}bytes5 constant cidPrefix${
    kind.name
  } = 0x01f1011b20;\n`;
};

const addFunctionHashKind = (protobuf, kind) => {
  protobuf.file += indentLevel(1);
  protobuf.file += `function hash${kind.name}(${codecClassName(
    kind
  )} memory _instance)`;
  protobuf.file += ` public view returns (bytes32) {\n`;

  protobuf.file += `${indentLevel(2)}bytes memory enc = ${codecName(
    kind
  )}.encode(_instance);\n`;
  protobuf.file += `${indentLevel(2)}bytes32 hash = keccak256(enc);\n`;
  protobuf.file += '\n';
  protobuf.file += `${indentLevel(2)}return hash;\n`;

  protobuf.file += `${indentLevel(1)}}\n`;
};

const addFunctionCalculateHashKind = (protobuf, kind) => {
  protobuf.file += indentLevel(1);
  protobuf.file += `function calculateHash${kind.name}(`;
  protobuf.file += kindParamsWithType(kind);
  protobuf.file += ') public view returns (bytes32) {\n';

  protobuf.file += paramsToInstance(kind);

  protobuf.file += `${indentLevel(2)}return hash${kind.name}(_instance);\n`;

  protobuf.file += `${indentLevel(1)}}\n`;
};

const addFunctionCalculateCidKind = (protobuf, kind) => {
  protobuf.file += indentLevel(1);
  protobuf.file += `function calculateCid${kind.name}(`;
  protobuf.file += kindParamsWithType(kind);
  protobuf.file += ') public view returns (bytes _cid) {\n';

  protobuf.file += paramsToInstance(kind);

  protobuf.file += `${indentLevel(2)}bytes32 _hash = hash${
    kind.name
  }(_instance);\n`;
  protobuf.file += `${indentLevel(2)}return cid.wrapInCid(cidPrefix${
    kind.name
  }, _hash);\n`;

  protobuf.file += `${indentLevel(1)}}\n`;
};

const addStorageField = (protobuf, kind) => {
  protobuf.file += indentLevel(1);
  protobuf.file += `mapping (bytes32 => ${kind.name}Codec.${kind.name})`;
  protobuf.file += ` private ${storageMapName(kind)};\n`;
};

const addKindStorageDelegateField = (protobuf, kind) => {
  protobuf.file += indentLevel(1);
  protobuf.file += `I${kind.name}Storage public ${storageDelegateName(
    kind
  )};\n`;
};

const addStoredEvent = (protobuf, kind) => {
  protobuf.file += indentLevel(1);
  protobuf.file += `event ${kind.name}Stored(bytes _cid);\n`;
};

const addFunctionStoreKind = (protobuf, kind) => {
  protobuf.file += indentLevel(1);
  protobuf.file += `function store${kind.name}(`;
  protobuf.file += kindParamsWithType(kind);
  protobuf.file += ') public returns (bytes) {\n';

  protobuf.file += paramsToInstance(kind);

  protobuf.file += `${indentLevel(2)}bytes32 hash = hash${
    kind.name
  }(_instance);\n`;

  protobuf.file += '\n';
  protobuf.file += `${indentLevel(2)}${storageMapName(
    kind
  )}[hash] = _instance;\n`;
  protobuf.file += '\n';

  protobuf.file += `${indentLevel(
    2
  )}bytes memory _cid = cid.wrapInCid(cidPrefix${kind.name}, hash);\n`;

  protobuf.file += `${indentLevel(2)}emit ${kind.name}Stored(_cid);\n`;

  protobuf.file += `${indentLevel(2)}return _cid;\n`;

  protobuf.file += `${indentLevel(1)}}\n`;
};

const addFunctionDelegateStoreKind = (protobuf, kind) => {
  protobuf.file += indentLevel(1);
  protobuf.file += `function store${kind.name}(`;
  protobuf.file += kindParamsWithType(kind);
  protobuf.file += ') public returns (bytes) {\n';

  protobuf.file += `${indentLevel(2)}bytes memory _cid = ${storageDelegateName(
    kind
  )}.store${kind.name}(${kindParams(kind)});\n`;

  protobuf.file += `${indentLevel(2)}emit ${kind.name}Stored(_cid);\n`;

  protobuf.file += `${indentLevel(2)}return _cid;\n`;

  protobuf.file += `${indentLevel(1)}}\n`;
};

const addFunctionInterfaceStoreKind = (protobuf, kind) => {
  protobuf.file += indentLevel(1);
  protobuf.file += `function store${kind.name}(`;
  protobuf.file += kindParamsWithType(kind);
  protobuf.file += ') public returns (bytes);\n';
};

const addFunctionRetrieveKind = (protobuf, kind) => {
  protobuf.file += indentLevel(1);
  protobuf.file += `function retrieve${kind.name}(bytes _cid)`;
  protobuf.file += ' external view returns (';
  protobuf.file += kindParamsWithType(kind);
  protobuf.file += ') {\n';

  protobuf.file += `${indentLevel(2)}bytes32 _hash = cid.unwrapCid(_cid);\n`;
  protobuf.file += `${indentLevel(2)}${codecClassName(
    kind
  )} memory _instance = ${storageMapName(kind)}[_hash];\n`;

  protobuf.file += indentLevel(2);
  protobuf.file += 'return (';
  protobuf.file += kind.fields
    .map(field => `_instance.${field.name}`)
    .join(', ');
  protobuf.file += ');\n';

  protobuf.file += `${indentLevel(1)}}\n`;
};

const addFunctionDelegateRetrieveKind = (protobuf, kind) => {
  protobuf.file += indentLevel(1);
  protobuf.file += `function retrieve${kind.name}(bytes _cid)`;
  protobuf.file += ' external view returns (';
  protobuf.file += kindParamsWithType(kind);
  protobuf.file += ') {\n';

  protobuf.file += indentLevel(2);
  protobuf.file += `return ${storageDelegateName(kind)}.retrieve${
    kind.name
  }(_cid);\n`;

  protobuf.file += `${indentLevel(1)}}\n`;
};

const addFunctionInterfaceRetrieveKind = (protobuf, kind) => {
  protobuf.file += indentLevel(1);
  protobuf.file += `function retrieve${kind.name}(bytes _cid)`;
  protobuf.file += ' external view returns (';
  protobuf.file += kindParamsWithType(kind);
  protobuf.file += ');\n';
};

const addOntologyStorageContract = (protobuf, grammar) => {
  const contractName = 'OntologyStorage';

  const addConstructor = (protobuf, grammar) => {
    protobuf.file += `${indentLevel(1)}constructor(`;
    protobuf.file += 'address[] _storage_delegate_addrs';
    protobuf.file += ') {\n';

    grammar.kinds.forEach((kind, i) => {
      protobuf.file += `${indentLevel(2)}${storageDelegateName(kind)} = I${
        kind.name
      }Storage(_storage_delegate_addrs[${i}]);\n`;
    });

    protobuf.file += `${indentLevel(1)}}\n`;
  };

  const addKindSection = (protobuf, kind) => {
    addFunctionDelegateStoreKind(protobuf, kind);
    protobuf.file += '\n';
    addFunctionDelegateRetrieveKind(protobuf, kind);
    protobuf.file += '\n';
  };

  protobuf.file += `contract ${contractName} is ${grammar.kinds
    .map(n => `I${n.name}Storage`)
    .join(', ')}{\n`;
  grammar.kinds.forEach(kind => addKindStorageDelegateField(protobuf, kind));
  protobuf.file += '\n';
  grammar.kinds.forEach(kind => addStoredEvent(protobuf, kind));
  protobuf.file += '\n';
  addConstructor(protobuf, grammar);
  protobuf.file += '\n';
  grammar.kinds.forEach(kind => addKindSection(protobuf, kind));
  protobuf.file += '}\n';
  protobuf.file += '\n';
};

const addKindStorageContract = (protobuf, kind) => {
  const contractName = `${kind.name}Storage`;

  protobuf.file += `contract ${contractName} is I${contractName} {\n`;
  addStorageField(protobuf, kind);
  protobuf.file += '\n';
  addCidConstant(protobuf, kind);
  protobuf.file += '\n';
  addStoredEvent(protobuf, kind);
  protobuf.file += '\n';
  addFunctionStoreKind(protobuf, kind);
  protobuf.file += '\n';
  addFunctionRetrieveKind(protobuf, kind);
  protobuf.file += '\n';
  addFunctionHashKind(protobuf, kind);
  protobuf.file += '\n';
  addFunctionCalculateCidKind(protobuf, kind);
  protobuf.file += '}\n';
  protobuf.file += '\n';
};

const addKindStorageContracts = (protobuf, grammar) => {
  grammar.kinds.forEach(kind => addKindStorageContract(protobuf, kind));
};

const addKindStorageInterfaceContract = (protobuf, kind) => {
  const contractName = `I${kind.name}Storage`;

  protobuf.file += `interface ${contractName} {\n`;
  addFunctionInterfaceStoreKind(protobuf, kind);
  protobuf.file += '\n';
  addFunctionInterfaceRetrieveKind(protobuf, kind);
  protobuf.file += '\n';
  protobuf.file += '}\n';
  protobuf.file += '\n';
};

const addKindStorageInterfaceContracts = (protobuf, grammar) => {
  grammar.kinds.forEach(kind =>
    addKindStorageInterfaceContract(protobuf, kind)
  );
};

module.exports = {
  addOntologyStorageContract,
  addKindStorageContracts,
  addKindStorageInterfaceContracts,
};
