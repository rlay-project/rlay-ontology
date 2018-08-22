#! /usr/bin/env bash
set -euxo pipefail

cd extract-grammar
npm run build
cp build/ontology_pb2.proto ../rlay_ontology/src/ontology.proto
