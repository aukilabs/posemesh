const fs = require('fs');
const generateInterfaceCXX = require('./generator-cxx');
const generateInterfaceObjC = require('./generator-objc');
const path = require('path');
const util = require('./util');

function validateInterfaceRecursive(path, interfaceName, json) {
  if (Array.isArray(json)) {
    let index = 0;
    for (const element of json) {
      validateInterfaceRecursive(`${path}[${index}]`, interfaceName, element);
      index++;
    }
  } else if (typeof json === 'object') {
    for (const key in json) {
      if (key.endsWith('.gen')) {
        continue;
      }
      let keyPath = path;
      if (keyPath.length > 0) {
        keyPath += '.';
      }
      if (key.includes('.')) {
        keyPath += '`' + key + '`';
      } else {
        keyPath += key;
      }
      if (typeof json[`${key}.gen`] !== 'boolean') {
        console.warn(`Unknown key '${keyPath}' in '${interfaceName}.json' interface JSON.`);
      }
      validateInterfaceRecursive(keyPath, interfaceName, json[key]);
    }
  }
}

function validateInterface(interfaceName, interfaceJson) {
  if (typeof interfaceJson !== 'object') {
    throw new Error(`Invalid '${interfaceName}.json' interface JSON.`);
  }
  validateInterfaceRecursive('', interfaceName, interfaceJson);
}

function generateInterface(interfaceName, interfaceJson) {
  generateInterfaceCXX(interfaceName, interfaceJson);
  generateInterfaceObjC(interfaceName, interfaceJson);
}

function generate() {
  const interfaceDirPath = path.resolve(__dirname, '..', 'interface');
  const interfaceFileNames = fs.readdirSync(interfaceDirPath, 'utf8');
  let interfaces = {};
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
      util.fillDestructor(interfaceJson);
      interfaces[interfaceName] = interfaceJson;
      validateInterface(interfaceName, interfaceJson);
    } catch (error) {
      console.error(`Failed to fill '${interfaceFileName}' interface JSON:\n`, error);
    }
  }
  for (const interfaceName in interfaces) {
    const interfaceJson = interfaces[interfaceName];
    try {
      generateInterface(interfaceName, interfaceJson);
      console.log(`Generated '${interfaceName}.json' interface code.`);
    } catch (error) {
      console.error(`Failed to generate '${interfaceName}.json' interface code:\n`, error);
    }
  }
}

generate();

module.exports = {};
