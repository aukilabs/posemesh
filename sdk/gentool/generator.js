const fs = require('fs');
const generateInterfaceC = require('./generator-c');
const generateInterfaceCXX = require('./generator-cxx');
const generateInterfaceJS = require('./generator-js');
const generateInterfaceObjC = require('./generator-objc');
const generateInterfaceSwift = require('./generator-swift');
const path = require('path');
const shared = require('./shared');
const util = require('./util');

const manualUmbrellaAndBridgingHeaderNames = new Set([
  'Config',
  'PoseEstimation',
  'Posemesh'
]);

const args = process.argv.slice(2);

if (args.length > 1) {
    console.error('Invalid usage.');
    process.exit(1);
    return;
}

const [option] = args;
let dontGitignoreCodeFiles = false;
if (typeof option !== 'undefined') {
  if (option !== 'dont-gitignore-code-files') {
    console.error('Invalid usage.');
    process.exit(1);
    return;
  }
  dontGitignoreCodeFiles = true;
}

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
  generateInterfaceC(interfaceName, interfaceJson);
  generateInterfaceCXX(interfaceName, interfaceJson);
  generateInterfaceJS(interfaceName, interfaceJson);
  generateInterfaceObjC(interfaceName, interfaceJson);
  generateInterfaceSwift(interfaceName, interfaceJson);
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
      util.fillEqualityOperator(interfaceJson);
      util.fillHashOperator(interfaceJson);
      util.fillCGenerateFuncAliasDefines(interfaceJson);
      interfaces[interfaceName] = interfaceJson;
      validateInterface(interfaceName, interfaceJson);
    } catch (error) {
      console.error(`Failed to fill '${interfaceFileName}' interface JSON:\n`, error);
    }
  }
  let gitignore = '# This file is automatically generated. Do not modify it manually as it will be overwritten!\n';
  gitignore += '\n';
  gitignore += '# CMake directories\n';
  gitignore += '/build/\n';
  gitignore += '/build-*/\n';
  gitignore += '/out/\n';
  gitignore += '/out-*/\n';
  gitignore += '\n';
  gitignore += '# CMake files\n';
  gitignore += '/cmake/GeneratedInterfaceFiles.cmake\n';
  if (!dontGitignoreCodeFiles) {
    gitignore += '\n';
    gitignore += '# Generated API 2 code\n';
    gitignore += '/platform/Web/API2.js\n';
    gitignore += '/platform/Web/src/API2.cpp\n';
    gitignore += '\n';
    gitignore += '# Generated Apple umbrella and bridging headers\n';
    gitignore += '/platform/Apple/include/Posemesh/Posemesh-Umbrella-Header.h\n';
    gitignore += '/platform/Apple/src/Posemesh-Bridging-Header.h\n';
  }
  let generatedCHeaders = new Set([]);
  let generatedCXXHeaders = new Set([]);
  let generatedCSources = new Set([]);
  let generatedCXXSources = new Set([]);
  let generatedObjCHeaders = new Set([]);
  let generatedObjCSources = new Set([]);
  let generatedSwiftSources = new Set([]);
  let generatedWebCXXSources = new Set([]);
  let generatedWebJSSources = new Set([]);
  let umbrellaHeaders = new Set([]);
  let bridgingHeaders = new Set([]);
  for (const headerName of manualUmbrellaAndBridgingHeaderNames) {
    umbrellaHeaders.add(`#import "${headerName}.h"`);
    bridgingHeaders.add(`#import <Posemesh/${headerName}.h>`);
  }
  for (const interfaceName in interfaces) {
    const interfaceJson = interfaces[interfaceName];
    try {
      generateInterface(interfaceName, interfaceJson);
      console.log(`Generated '${interfaceName}.json' interface code.`);
    } catch (error) {
      console.error(`Failed to generate '${interfaceName}.json' interface code:\n`, error);
    }

    if (!dontGitignoreCodeFiles) {
      gitignore += '\n';
      gitignore += `# Generated ${interfaceName} files\n`;

      // C
      gitignore += `/include/Posemesh/C/${interfaceName}.h\n`;
      gitignore += `/src/C/${interfaceName}.cpp\n`;

      // CXX
      gitignore += `/include/Posemesh/${interfaceName}.hpp\n`;
      gitignore += `/src/${interfaceName}.gen.cpp\n`;

      // JS
      gitignore += `/platform/Web/transform-typescript-definition-${interfaceName}.js\n`;
      gitignore += `/platform/Web/${interfaceName}.js\n`;
      gitignore += `/platform/Web/src/${interfaceName}.cpp\n`;

      // ObjC
      gitignore += `/platform/Apple/include/Posemesh/${interfaceName}.h\n`;
      gitignore += `/platform/Apple/src/${interfaceName}.mm\n`;

      // Swift
      gitignore += `/platform/Apple/src/${interfaceName}.swift\n`;
    }

    // Generated files
    generatedCHeaders.add(`/include/Posemesh/C/${interfaceName}.h`);
    generatedCXXHeaders.add(`/include/Posemesh/${interfaceName}.hpp`);
    generatedCSources.add(`/src/C/${interfaceName}.cpp`);
    generatedCXXSources.add(`/src/${interfaceName}.gen.cpp`);
    generatedObjCHeaders.add(`/platform/Apple/include/Posemesh/${interfaceName}.h`);
    generatedObjCSources.add(`/platform/Apple/src/${interfaceName}.mm`);
    generatedSwiftSources.add(`/platform/Apple/src/${interfaceName}.swift`);
    generatedWebCXXSources.add(`/platform/Web/src/${interfaceName}.cpp`);
    generatedWebJSSources.add(`/platform/Web/${interfaceName}.js`);
    umbrellaHeaders.add(`#import "${interfaceName}.h"`);
    bridgingHeaders.add(`#import <Posemesh/${interfaceName}.h>`);
  }
  gitignore += '\n';
  gitignore += '# This file\n';
  gitignore += '/.gitignore\n';
  fs.writeFileSync(path.resolve(__dirname, '..', '.gitignore'), gitignore, 'utf8');

  let generatedInterfaceFilesCMakeContent = '# This file is automatically generated. Do not modify it manually as it will be overwritten!\n';

  // C headers
  generatedInterfaceFilesCMakeContent += '\n';
  generatedInterfaceFilesCMakeContent += 'list(\n';
  generatedInterfaceFilesCMakeContent += '    APPEND POSEMESH_GENERATED_C_HEADERS\n';
  for (const generatedCHeader of Array.from(generatedCHeaders).sort()) {
    generatedInterfaceFilesCMakeContent += `        "\${CMAKE_CURRENT_LIST_DIR}/..${generatedCHeader}"\n`;
  }
  generatedInterfaceFilesCMakeContent += ')\n';

  // CXX headers
  generatedInterfaceFilesCMakeContent += '\n';
  generatedInterfaceFilesCMakeContent += 'list(\n';
  generatedInterfaceFilesCMakeContent += '    APPEND POSEMESH_GENERATED_CXX_HEADERS\n';
  for (const generatedCXXHeader of Array.from(generatedCXXHeaders).sort()) {
    generatedInterfaceFilesCMakeContent += `        "\${CMAKE_CURRENT_LIST_DIR}/..${generatedCXXHeader}"\n`;
  }
  generatedInterfaceFilesCMakeContent += ')\n';

  // C sources
  generatedInterfaceFilesCMakeContent += '\n';
  generatedInterfaceFilesCMakeContent += 'list(\n';
  generatedInterfaceFilesCMakeContent += '    APPEND POSEMESH_GENERATED_C_SOURCES\n';
  for (const generatedCSource of Array.from(generatedCSources).sort()) {
    generatedInterfaceFilesCMakeContent += `        "\${CMAKE_CURRENT_LIST_DIR}/..${generatedCSource}"\n`;
  }
  generatedInterfaceFilesCMakeContent += ')\n';

  // CXX sources
  generatedInterfaceFilesCMakeContent += '\n';
  generatedInterfaceFilesCMakeContent += 'list(\n';
  generatedInterfaceFilesCMakeContent += '    APPEND POSEMESH_GENERATED_CXX_SOURCES\n';
  for (const generatedCXXSource of Array.from(generatedCXXSources).sort()) {
    generatedInterfaceFilesCMakeContent += `        "\${CMAKE_CURRENT_LIST_DIR}/..${generatedCXXSource}"\n`;
  }
  generatedInterfaceFilesCMakeContent += ')\n';

  // ObjC headers
  generatedInterfaceFilesCMakeContent += '\n';
  generatedInterfaceFilesCMakeContent += 'list(\n';
  generatedInterfaceFilesCMakeContent += '    APPEND POSEMESH_GENERATED_OBJC_HEADERS\n';
  for (const generatedObjCHeader of Array.from(generatedObjCHeaders).sort()) {
    generatedInterfaceFilesCMakeContent += `        "\${CMAKE_CURRENT_LIST_DIR}/..${generatedObjCHeader}"\n`;
  }
  generatedInterfaceFilesCMakeContent += ')\n';

  // ObjC sources
  generatedInterfaceFilesCMakeContent += '\n';
  generatedInterfaceFilesCMakeContent += 'list(\n';
  generatedInterfaceFilesCMakeContent += '    APPEND POSEMESH_GENERATED_OBJC_SOURCES\n';
  for (const generatedObjCSource of Array.from(generatedObjCSources).sort()) {
    generatedInterfaceFilesCMakeContent += `        "\${CMAKE_CURRENT_LIST_DIR}/..${generatedObjCSource}"\n`;
  }
  generatedInterfaceFilesCMakeContent += ')\n';

  // Swift sources
  generatedInterfaceFilesCMakeContent += '\n';
  generatedInterfaceFilesCMakeContent += 'list(\n';
  generatedInterfaceFilesCMakeContent += '    APPEND POSEMESH_GENERATED_SWIFT_SOURCES\n';
  for (const generatedSwiftSource of Array.from(generatedSwiftSources).sort()) {
    generatedInterfaceFilesCMakeContent += `        "\${CMAKE_CURRENT_LIST_DIR}/..${generatedSwiftSource}"\n`;
  }
  generatedInterfaceFilesCMakeContent += ')\n';

  // Web CXX sources
  generatedInterfaceFilesCMakeContent += '\n';
  generatedInterfaceFilesCMakeContent += 'list(\n';
  generatedInterfaceFilesCMakeContent += '    APPEND POSEMESH_GENERATED_WEB_CXX_SOURCES\n';
  for (const generatedWebCXXSource of Array.from(generatedWebCXXSources).sort()) {
    generatedInterfaceFilesCMakeContent += `        "\${CMAKE_CURRENT_LIST_DIR}/..${generatedWebCXXSource}"\n`;
  }
  generatedInterfaceFilesCMakeContent += ')\n';

  // Web JS sources
  generatedInterfaceFilesCMakeContent += '\n';
  generatedInterfaceFilesCMakeContent += 'list(\n';
  generatedInterfaceFilesCMakeContent += '    APPEND POSEMESH_GENERATED_WEB_JS_SOURCES\n';
  for (const generatedWebJSSource of Array.from(generatedWebJSSources).sort()) {
    generatedInterfaceFilesCMakeContent += `        "\${CMAKE_CURRENT_LIST_DIR}/..${generatedWebJSSource}"\n`;
  }
  generatedInterfaceFilesCMakeContent += ')\n';

  util.writeFileContentIfDifferent(path.resolve(__dirname, '..', 'cmake', 'GeneratedInterfaceFiles.cmake'), generatedInterfaceFilesCMakeContent);

  let umbrellaHeaderContent = '/* This file is automatically generated. Do not modify it manually as it will be overwritten! */\n';
  umbrellaHeaderContent += '\n';
  for (const umbrellaHeader of Array.from(umbrellaHeaders).sort()) {
    umbrellaHeaderContent += `${umbrellaHeader}\n`;
  }
  util.writeFileContentIfDifferent(path.resolve(__dirname, '..', 'platform', 'Apple', 'include', 'Posemesh', 'Posemesh-Umbrella-Header.h'), umbrellaHeaderContent);

  let bridgingHeaderContent = '/* This file is automatically generated. Do not modify it manually as it will be overwritten! */\n';
  bridgingHeaderContent += '\n';
  for (const bridgingHeader of Array.from(bridgingHeaders).sort()) {
    bridgingHeaderContent += `${bridgingHeader}\n`;
  }
  util.writeFileContentIfDifferent(path.resolve(__dirname, '..', 'platform', 'Apple', 'src', 'Posemesh-Bridging-Header.h'), bridgingHeaderContent);

  let api2CXXSource = '/* This file is automatically generated. Do not modify it manually as it will be overwritten! */\n';
  api2CXXSource += '\n';
  for (const interfaceName of Array.from(shared.requiredVectorsOfClasses).sort()) {
    api2CXXSource += `#include <Posemesh/${interfaceName}.hpp>\n`;
  }
  api2CXXSource += '#include <emscripten/bind.h>\n';
  api2CXXSource += '#include <memory>\n';
  api2CXXSource += '\n';
  api2CXXSource += 'using namespace emscripten;\n';
  api2CXXSource += '\n';
  api2CXXSource += 'EMSCRIPTEN_BINDINGS(API2)\n';
  api2CXXSource += '{\n';
  for (const interfaceName of Array.from(shared.requiredVectorsOfClasses).sort()) {
    api2CXXSource += `    register_vector<std::shared_ptr<psm::${interfaceName}>>("Vector${interfaceName}");\n`;
  }
  api2CXXSource += '}\n';
  util.writeFileContentIfDifferent(path.resolve(__dirname, '..', 'platform', 'Web', 'src', 'API2.cpp'), api2CXXSource);

  let api2JSSource = '/* This file is automatically generated. Do not modify it manually as it will be overwritten! */\n';
  for (const interfaceName of Array.from(shared.requiredVectorsOfClasses).sort()) {
    api2JSSource += '\n';

    api2JSSource += `__internalPosemeshAPI.fromVector${interfaceName} = function(vector${interfaceName}, allowNullItems = true) {\n`;
    api2JSSource += `    let size = vector${interfaceName}.size();\n`;
    api2JSSource += `    let array = new Array(size);\n`;
    api2JSSource += `    try {\n`;
    api2JSSource += `        for (let i = 0; i < size; ++i) {\n`;
    api2JSSource += `            array[i] = vector${interfaceName}.get(i);\n`;
    api2JSSource += `            if (!array[i]) {\n`;
    api2JSSource += `                array[i] = null;\n`;
    api2JSSource += `                if (!allowNullItems) {\n`;
    api2JSSource += `                    throw new Error(\`Array item at index \${i} is null.\`);\n`;
    api2JSSource += `                }\n`;
    api2JSSource += `            }\n`;
    api2JSSource += `        }\n`;
    api2JSSource += `        return array;\n`;
    api2JSSource += `    } catch (error) {\n`;
    api2JSSource += `        for (let item of array) {\n`;
    api2JSSource += `            if (item) {\n`;
    api2JSSource += `                item.delete();\n`;
    api2JSSource += `            }\n`;
    api2JSSource += `        }\n`;
    api2JSSource += `        throw error;\n`;
    api2JSSource += `    }\n`;
    api2JSSource += `}\n`;

    api2JSSource += `__internalPosemeshAPI.toVector${interfaceName} = function(array, allowNullItems = true) {\n`;
    api2JSSource += `    let vector${interfaceName} = new __internalPosemesh.Vector${interfaceName}();\n`;
    api2JSSource += `    try {\n`;
    api2JSSource += `        vector${interfaceName}.resize(array.length, null);\n`;
    api2JSSource += `        let i = 0;\n`;
    api2JSSource += `        for (let item of array) {\n`;
    api2JSSource += `            if (typeof item === 'undefined' || item === null) {\n`;
    api2JSSource += `                if (!allowNullItems) {\n`;
    api2JSSource += `                    throw new Error(\`Array item at index \${i} is null.\`);\n`;
    api2JSSource += `                }\n`;
    api2JSSource += `                i++;\n`;
    api2JSSource += `                continue;\n`;
    api2JSSource += `            }\n`;
    api2JSSource += `            if (item instanceof __internalPosemesh.${interfaceName}) {\n`;
    api2JSSource += `                vector${interfaceName}.set(i, item);\n`;
    api2JSSource += `                i++;\n`;
    api2JSSource += `                continue;\n`;
    api2JSSource += `            }\n`;
    api2JSSource += `            throw new Error(\`Array item at index \${i} is not an instance of ${interfaceName} class.\`);\n`;
    api2JSSource += `        }\n`;
    api2JSSource += `        return vector${interfaceName};\n`;
    api2JSSource += `    } catch (error) {\n`;
    api2JSSource += `        vector${interfaceName}.delete();\n`;
    api2JSSource += `        throw error;\n`;
    api2JSSource += `    }\n`;
    api2JSSource += `}\n`;
  }
  util.writeFileContentIfDifferent(path.resolve(__dirname, '..', 'platform', 'Web', 'API2.js'), api2JSSource);
}

generate();

module.exports = {};
