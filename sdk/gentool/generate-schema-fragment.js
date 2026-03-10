/**
 * Generates gentool.schema.generated.json with enum values for parameterized types
 * (CLASS_MIX:Vector3, ENUM:SolvePnPMethod, etc.) so editors can autocomplete them.
 *
 * Run as part of `npm run generate`.
 */
const fs = require('fs');
const path = require('path');
const shared = require('./shared');

const PRIMITIVES = [
  'int8', 'int16', 'int32', 'int64',
  'uint8', 'uint16', 'uint32', 'uint64',
  'float', 'double', 'boolean', 'string'
];

const CLASS_PREFIXES = ['CLASS', 'CLASS_REF', 'CLASS_MIX', 'CLASS_PTR', 'CLASS_PTR_REF', 'CLASS_PTR_MIX'];
const ARRAY_PREFIXES = ['ARRAY', 'ARRAY_REF', 'ARRAY_MIX', 'ARRAY_PTR', 'ARRAY_PTR_REF', 'ARRAY_PTR_MIX'];

const PARAM_CLASS_PREFIXES = ['CLASS', 'CLASS_REF', 'CLASS_PTR', 'CLASS_PTR_REF'];
const PARAM_ARRAY_PREFIXES = ['ARRAY', 'ARRAY_REF', 'ARRAY_PTR', 'ARRAY_PTR_REF'];

function collectNames(dir, subdir = '') {
  const dirPath = path.resolve(__dirname, '..', dir, subdir);
  if (!fs.existsSync(dirPath)) return [];
  return fs.readdirSync(dirPath, 'utf8')
    .filter((f) => f.toLowerCase().endsWith('.json'))
    .map((f) => path.basename(f, '.json'));
}

function main() {
  const enumNames = collectNames('enum');
  if (!shared.ignoreCompileTests && fs.existsSync(path.resolve(__dirname, '..', 'enum', 'CompileTests'))) {
    enumNames.push(...collectNames('enum', 'CompileTests'));
  }

  const interfaceNames = collectNames('interface');
  if (!shared.ignoreCompileTests && fs.existsSync(path.resolve(__dirname, '..', 'interface', 'CompileTests'))) {
    interfaceNames.push(...collectNames('interface', 'CompileTests'));
  }

  const allTypeNames = [...new Set([...interfaceNames, ...enumNames])];

  const typeKeyEnum = [];
  for (const prefix of CLASS_PREFIXES) {
    for (const name of interfaceNames) {
      typeKeyEnum.push(`${prefix}:${name}`);
    }
  }
  for (const name of enumNames) {
    typeKeyEnum.push(`ENUM:${name}`);
  }
  for (const prefix of ARRAY_PREFIXES) {
    for (const p of PRIMITIVES) {
      typeKeyEnum.push(`${prefix}:${p}`);
    }
    for (const name of allTypeNames) {
      typeKeyEnum.push(`${prefix}:${name}`);
    }
  }

  const paramTypeKeyEnum = [];
  for (const prefix of PARAM_CLASS_PREFIXES) {
    for (const name of interfaceNames) {
      paramTypeKeyEnum.push(`${prefix}:${name}`);
    }
  }
  for (const name of enumNames) {
    paramTypeKeyEnum.push(`ENUM:${name}`);
  }
  for (const prefix of PARAM_ARRAY_PREFIXES) {
    for (const p of PRIMITIVES) {
      paramTypeKeyEnum.push(`${prefix}:${p}`);
    }
    for (const name of allTypeNames) {
      paramTypeKeyEnum.push(`${prefix}:${name}`);
    }
  }

  const generated = {
    '$schema': 'https://json-schema.org/draft/2020-12/schema',
    '$id': 'https://posemesh.auki.dev/schemas/gentool.schema.generated.json',
    'title': 'Generated type enums for GenTool schema',
    '$defs': {
      'ParameterizedTypeKey': {
        'type': 'string',
        'enum': typeKeyEnum.sort()
      },
      'ParameterizedParamTypeKey': {
        'type': 'string',
        'enum': paramTypeKeyEnum.sort()
      }
    }
  };

  const outPath = path.resolve(__dirname, 'gentool.schema.generated.json');
  const content = JSON.stringify(generated, null, 2) + '\n';
  fs.writeFileSync(outPath, content, 'utf8');
  console.log('Generated gentool.schema.generated.json');
}

main();
