const fs = require('fs');
const path = require('path');
const util = require('./util');

function generateCppSource(interfaceName, interfaceJson) {
  const name = util.getLangClassName(interfaceJson, util.JS);
  const nameCxx = util.getLangClassName(interfaceJson, util.CXX);
  const static = util.getClassStatic(interfaceJson);
  const copyable = util.getClassCopyable(interfaceJson);

  let includesFirst = new Set([`#include <Posemesh/${interfaceName}.hpp>`, '#include <emscripten/bind.h>', '#include <memory>']), includesSecond = new Set([]);
  let unnamedNamespace = '';

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += '%INCLUDES%\n';
  code += 'using namespace emscripten;\n';
  code += 'using namespace psm;\n';
  code += '%UNNAMED_NAMESPACE%\n';

  code += `EMSCRIPTEN_BINDINGS(${name})\n`;
  code += `{\n`;
  code += `    class_<${nameCxx}>("${name}")`;

  const smartPtrLine = `\n        .smart_ptr<std::shared_ptr<${nameCxx}>>("${name}")`;
  let smartPtrAdded = false;

  const parameterlessConstructor = util.getClassParameterlessConstructor(interfaceJson);
  const pCtorDefinition = util.getConstructorDefinition(parameterlessConstructor);
  const pCtorVisibility = util.getConstructorVisibility(parameterlessConstructor);
  if (!static && pCtorDefinition !== util.ConstructorDefinition.deleted) {
    if (!smartPtrAdded) {
      code += smartPtrLine;
      smartPtrAdded = true;
    }
    if (pCtorVisibility === util.Visibility.public) {
        code += `\n        .constructor(&std::make_shared<${nameCxx}>)`;
    }
  }

  let needsDuplicateMethod = false;

  const copyConstructor = util.getClassCopyConstructor(interfaceJson);
  const cCtorDefinition = util.getConstructorDefinition(copyConstructor);
  const cCtorVisibility = util.getConstructorVisibility(copyConstructor);
  if (!static && copyable && cCtorDefinition !== util.ConstructorDefinition.deleted) {
    if (!smartPtrAdded) {
      code += smartPtrLine;
      smartPtrAdded = true;
    }
    if (cCtorVisibility === util.Visibility.public) {
      code += `\n        .constructor(&std::make_shared<${nameCxx}, const ${nameCxx}&>)`;
      needsDuplicateMethod = true;
    }
  }

  // TODO: other constructors

  if (needsDuplicateMethod) {
    code += `\n        .function("duplicate()", &std::make_shared<${nameCxx}, const ${nameCxx}&>, nonnull<ret_val>())`;
  }

  const equalityOperator = interfaceJson.equalityOperator;
  if (equalityOperator.defined) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.camelBack);
    code += `\n        .function("equals(${mainArgName})", &${nameCxx}::operator==)`;
  }

  const hashOperator = interfaceJson.hashOperator;
  if (hashOperator.defined) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.camelBack);
    code += `\n        .function("hash()", &hash)`;
    if (unnamedNamespace.length > 0) {
      unnamedNamespace += '\n';
    }
    unnamedNamespace += `std::size_t hash(const ${nameCxx}& ${mainArgName}) noexcept\n`;
    unnamedNamespace += `{\n`;
    unnamedNamespace += `    return std::hash<${nameCxx}> {}(${mainArgName});\n`;
    unnamedNamespace += `}\n`;
  }

  for (const propertyJson of util.getProperties(interfaceJson)) {
    const propStatic = util.getPropertyStatic(propertyJson);
    if (propertyJson.hasGetter) {
      const getterName = util.getPropertyGetterName(propertyJson, util.JS);
      const getterNameCxx = util.getPropertyGetterName(propertyJson, util.CXX);
      const getterVisibility = util.getPropertyGetterVisibility(propertyJson);
      if (getterVisibility === util.Visibility.public) {
        if (propStatic) {
          code += `\n        .class_function("__${getterName}()", &${nameCxx}::${getterNameCxx})`;
        } else {
          code += `\n        .function("__${getterName}()", &${nameCxx}::${getterNameCxx})`;
        }
      }
    }
    if (propertyJson.hasSetter) {
      const setterName = util.getPropertySetterName(propertyJson, util.JS);
      const setterNameCxx = util.getPropertySetterName(propertyJson, util.CXX);
      const setterArgName = util.getPropertySetterArgName(propertyJson, util.JS);
      const setterVisibility = util.getPropertySetterVisibility(propertyJson);
      if (setterVisibility === util.Visibility.public) {
        if (propStatic) {
          code += `\n        .class_function("__${setterName}(${setterArgName})", &${nameCxx}::${setterNameCxx})`;
        } else {
          code += `\n        .function("__${setterName}(${setterArgName})", &${nameCxx}::${setterNameCxx})`;
        }
      }
    }
  }

  code += `;\n`;
  code += `}\n`;

  includesFirst = Array.from(includesFirst).sort();
  includesSecond = Array.from(includesSecond).sort();
  let includes = '';
  if (includesFirst.length > 0) {
    includes += '\n';
    for (const include of includesFirst) {
      includes += include + '\n';
    }
  }
  if (includesSecond.length > 0) {
    includes += '\n';
    for (const include of includesSecond) {
      includes += include + '\n';
    }
  }
  code = code.replaceAll('%INCLUDES%', includes);
  if (unnamedNamespace.length > 0) {
    unnamedNamespace = `\nnamespace {\n${unnamedNamespace}}\n`;
  }
  code = code.replaceAll('%UNNAMED_NAMESPACE%', unnamedNamespace);

  return code;
}

function generateJsSource(interfaceName, interfaceJson) {
  const name = util.getLangClassName(interfaceJson, util.JS);

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  let builderFunctionBody = '';

  for (const propertyJson of util.getProperties(interfaceJson)) {
    const propName = util.getPropertyName(propertyJson, util.JS);
    const propStatic = util.getPropertyStatic(propertyJson);
    const propRootObj = `${name === 'Posemesh' ? 'Posemesh' : `Posemesh.${name}`}${propStatic ? '' : '.prototype'}`;
    let propDef = `    Object.defineProperty(${propRootObj}, '${propName}', {\n`;
    let addPropDef = false;
    if (propertyJson.hasGetter) {
      const getterName = util.getPropertyGetterName(propertyJson, util.JS);
      const getterVisibility = util.getPropertyGetterVisibility(propertyJson);
      if (getterVisibility === util.Visibility.public) {
        addPropDef = true;
        propDef += `        get: ${propRootObj}.__${getterName},\n`;
      }
    }
    if (propertyJson.hasSetter) {
      const setterName = util.getPropertySetterName(propertyJson, util.JS);
      const setterVisibility = util.getPropertySetterVisibility(propertyJson);
      if (setterVisibility === util.Visibility.public) {
        addPropDef = true;
        propDef += `        set: ${propRootObj}.__${setterName},\n`;
      }
    }
    propDef += `        enumerable: true,\n`;
    propDef += `        configurable: false\n`;
    propDef += '    });\n'
    if (addPropDef) {
      if (builderFunctionBody.length > 0) {
        builderFunctionBody += '\n';
      }
      builderFunctionBody += propDef;
    }
  }

  const aliases = util.getLangAliases(interfaceJson, util.JS);
  if (aliases.length > 0) {
    if (builderFunctionBody.length > 0) {
      builderFunctionBody += '\n';
    }
    for (const alias of aliases) {
      if (alias === 'Posemesh') {
        throw new Error(`Alias of a class should not be 'Posemesh'.`);
      }
      if (name === 'Posemesh') {
        builderFunctionBody += `    Posemesh.${alias} = Posemesh;\n`;
      } else {
        builderFunctionBody += `    Posemesh.${alias} = Posemesh.${name};\n`;
      }
    }
  }

  code += `\n`;
  if (builderFunctionBody.length > 0) {
    code += `__internalPosemeshAPI.builderFunctions.push(function() {\n`;
    code += builderFunctionBody;
    code += `});\n`;
  } else {
    code += `__internalPosemeshAPI.builderFunctions.push(function() { /* Do nothing. */ });\n`;
  }
  return code;
}

function generateTransformTsDefScript(interfaceName, interfaceJson) {
  const name = util.getLangClassName(interfaceJson, util.JS);
  const fixFuncName = `fix${name}`;
  const isSpecialPosemeshClass = name === 'Posemesh';
  const static = util.getClassStatic(interfaceJson);
  const copyable = util.getClassCopyable(interfaceJson);

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += `\n`;
  code += `function ${fixFuncName}(content, newLine, tab) {\n`;

  // TODO: impl
  code += `    // TODO: impl\n`;

  code += `}\n`;
  code += `\n`;
  code += `module.exports = ${fixFuncName};\n`;
  return code;
}

function generateInterfaceJS(interfaceName, interfaceJson) {
  const cppSourceFilePath = path.resolve(__dirname, '..', 'platform', 'Web', 'src', `${interfaceName}.cpp`);
  const jsSourceFilePath = path.resolve(__dirname, '..', 'platform', 'Web', `${interfaceName}.js`);
  const transformTsDefScriptFilePath = path.resolve(__dirname, '..', 'platform', 'Web', `transform-typescript-definition-${interfaceName}.js`);

  let cppSourceCode = generateCppSource(interfaceName, interfaceJson);
  let jsSourceCode = generateJsSource(interfaceName, interfaceJson);
  let transformTsDefScriptCode = generateTransformTsDefScript(interfaceName, interfaceJson);

  fs.writeFileSync(cppSourceFilePath, cppSourceCode, 'utf8');
  fs.writeFileSync(jsSourceFilePath, jsSourceCode, 'utf8');
  fs.writeFileSync(transformTsDefScriptFilePath, transformTsDefScriptCode, 'utf8');
}

module.exports = generateInterfaceJS;
