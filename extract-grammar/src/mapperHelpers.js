const assert = require('assert');

const annotationField = param => {
  assert(param === 'annotationAnnotations');
  return {
    name: 'annotations',
    kind: param,
  };
};

module.exports = {
  annotationField,
};
