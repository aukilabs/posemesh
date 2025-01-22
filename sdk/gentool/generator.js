const fs = require('fs');
const generateInterfaceCXX = require('./generator-cxx');
const path = require('path');
const util = require('./util');

function generateInterface(interfaceName, interfaceJson) {
  generateInterfaceCXX(interfaceName, interfaceJson);
}

function generate() {
  const interfaceDirPath = path.resolve(__dirname, '..', 'interface');
  const interfaceFileNames = fs.readdirSync(interfaceDirPath, 'utf8');
  for (const interfaceFileName of interfaceFileNames) {
    if (!interfaceFileName.toLowerCase().endsWith('.json')) {
      continue;
    }
    const interfaceName = interfaceFileName.substring(0, interfaceFileName.length - 5);
    const interfaceFilePath = path.resolve(interfaceDirPath, interfaceFileName);
    const interfaceFileContent = fs.readFileSync(interfaceFilePath, 'utf8');
    try {
      const interfaceJson = JSON.parse(interfaceFileContent);
      util.fillClassName(interfaceJson);
      util.fillClassStatic(interfaceJson);
      util.fillClassFinal(interfaceJson);
      util.fillAliases(interfaceJson);
      util.fillHeaderGuardName(interfaceJson);
      util.fillProperties(interfaceJson);
      util.fillCopyable(interfaceJson);
      util.fillMovable(interfaceJson);
      util.fillParameterlessConstructor(interfaceJson);
      util.fillCopyConstructor(interfaceJson);
      util.fillMoveConstructor(interfaceJson);
      generateInterface(interfaceName, interfaceJson);
      console.log(`Generated '${interfaceFileName}' interface code.`);
    } catch (error) {
      console.log(`Failed to generate '${interfaceFileName}' interface code:\n`, error);
    }
  }
}

generate();

module.exports = {};
