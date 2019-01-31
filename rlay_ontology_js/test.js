const { getEntityCid } = require('./pkg/rlay_ontology_js_nodejs');

const result = getEntityCid({ "type": "Individual", "data_property_assertions": ["0x019580031b20567c6c54ad4525f1529268a90c0633377596697338a48d36624f180f73b46959"]}
);
console.log(result);
