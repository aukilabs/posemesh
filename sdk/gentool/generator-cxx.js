const fs = require('fs');
const path = require('path');
const util = require('./util');

function generateHeader(interfaceName, interfaceJson) {
  const name = util.getLangClassName(interfaceJson, util.CXX);
  const aliases = util.getLangAliases(interfaceJson, util.CXX);
  const headerGuardName = util.getHeaderGuardName(interfaceJson);
  const headerGuard = `__POSEMESH_${headerGuardName}_HPP__`;

  let code = `#ifndef ${headerGuard}\n`;
  code += `#define ${headerGuard}\n`;
  code += '\n';
  code += '#include "API.hpp"\n';
  code += '\n';
  code += 'namespace psm {\n';
  code += '\n';
  code += `class ${name} {\n`;
  code += '};\n';
  for (const alias of aliases) {
    code += `using ${alias} = ${name};\n`;
  }
  code += '\n';
  code += '}\n';
  code += '\n';
  code += `#endif // ${headerGuard}\n`;
  return code;
}

function generateSource(interfaceName, interfaceJson) {
  let code = '';
  return code;
}

function generateInterfaceCXX(interfaceName, interfaceJson) {
  const headerFilePath = path.resolve(__dirname, '..', 'include', 'Posemesh', `${interfaceName}.hpp`);
  const sourceFilePath = path.resolve(__dirname, '..', 'src', `${interfaceName}.gen.cpp`);

  let headerCode = generateHeader(interfaceName, interfaceJson);
  let sourceCode = generateSource(interfaceName, interfaceJson);

  fs.writeFileSync(headerFilePath, headerCode, 'utf8');
  fs.writeFileSync(sourceFilePath, sourceCode, 'utf8');
}

module.exports = generateInterfaceCXX;
