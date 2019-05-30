const fs = require('fs');
const path = require('path');

const packageJsonPath = path.join(__dirname, './pkg/package.json');

const contents = JSON.parse(fs.readFileSync(packageJsonPath));
// set name
contents.name = '@rlay/ontology';
contents.files.push("rlay_ontology_js_nodejs_bg.js");

fs.writeFileSync(packageJsonPath, JSON.stringify(contents, null, 4));
