const path = require('path');
const util = require('./util');

function generateCppSource(enums, interfaces, interfaceName, interfaceJson) {
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

  const toStringOperator = interfaceJson.toStringOperator;
  if (toStringOperator.defined) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.camelBack);
    code += `\n        .function("toString()", &toString)`;
    if (unnamedNamespace.length > 0) {
      unnamedNamespace += '\n';
    }
    unnamedNamespace += `std::string toString(const ${nameCxx}& ${mainArgName})\n`;
    unnamedNamespace += `{\n`;
    unnamedNamespace += `    return static_cast<std::string>(${mainArgName});\n`;
    unnamedNamespace += `}\n`;
  }

  for (const propertyJson of util.getProperties(interfaceJson)) {
    const propStatic = util.getPropertyStatic(propertyJson);
    if (propertyJson.hasGetter) {
      const getterName = util.getPropertyGetterName(propertyJson, util.JS);
      const getterNameCxx = util.getPropertyGetterName(propertyJson, util.CXX);
      const getterVisibility = util.getPropertyGetterVisibility(propertyJson);
      if (getterVisibility === util.Visibility.public) {
        let funcName = `${nameCxx}::${getterNameCxx}`;
        if (util.isClassType(propertyJson.type) || util.isClassRefType(propertyJson.type) || util.isClassMixType(propertyJson.type)) {
          funcName = getterNameCxx;
          const propTypeRawWithoutPfx = propertyJson.type.split(':').slice(1).join(':');
          const propTypeInterfaceJson = interfaces[propTypeRawWithoutPfx];
          if (typeof propTypeInterfaceJson === 'undefined') {
            throw new Error(`Unknown class: ${propTypeRawWithoutPfx}`);
          }
          if (unnamedNamespace.length > 0) {
            unnamedNamespace += '\n';
          }
          let selfArg = '';
          if (!propStatic) {
            if (propertyJson.getterConst) {
              selfArg = `const ${nameCxx}& self`;
            } else {
              selfArg = `${nameCxx}& self`;
            }
          }
          unnamedNamespace += `std::shared_ptr<psm::${util.getLangClassName(propTypeInterfaceJson, util.CXX)}> ${funcName}(${selfArg})\n`;
          unnamedNamespace += `{\n`;
          if (propStatic) {
            if (util.isClassType(propertyJson.type)) {
              unnamedNamespace += `    return std::make_shared<psm::${util.getLangClassName(propTypeInterfaceJson, util.CXX)}>(std::move(${nameCxx}::${getterNameCxx}()));\n`;
              includesFirst.add('#include <utility>');
            } else {
              unnamedNamespace += `    return std::make_shared<psm::${util.getLangClassName(propTypeInterfaceJson, util.CXX)}>(${nameCxx}::${getterNameCxx}());\n`;
            }
          } else {
            if (util.isClassType(propertyJson.type)) {
              unnamedNamespace += `    return std::make_shared<psm::${util.getLangClassName(propTypeInterfaceJson, util.CXX)}>(std::move(self.${getterNameCxx}()));\n`;
              includesFirst.add('#include <utility>');
            } else {
              unnamedNamespace += `    return std::make_shared<psm::${util.getLangClassName(propTypeInterfaceJson, util.CXX)}>(self.${getterNameCxx}());\n`;
            }
          }
          unnamedNamespace += `}\n`;
        }
        if (propStatic) {
          code += `\n        .class_function("__${getterName}()", &${funcName})`;
        } else {
          code += `\n        .function("__${getterName}()", &${funcName})`;
        }
      }
    }
    if (propertyJson.hasSetter) {
      const setterName = util.getPropertySetterName(propertyJson, util.JS);
      const setterNameCxx = util.getPropertySetterName(propertyJson, util.CXX);
      const setterArgName = util.getPropertySetterArgName(propertyJson, util.JS);
      const setterVisibility = util.getPropertySetterVisibility(propertyJson);
      if (setterVisibility === util.Visibility.public) {
        let funcName = `${nameCxx}::${setterNameCxx}`;
        if (util.isClassType(propertyJson.type) || util.isClassRefType(propertyJson.type) || util.isClassMixType(propertyJson.type)) {
          funcName = setterNameCxx;
          const propTypeRawWithoutPfx = propertyJson.type.split(':').slice(1).join(':');
          const propTypeInterfaceJson = interfaces[propTypeRawWithoutPfx];
          if (typeof propTypeInterfaceJson === 'undefined') {
            throw new Error(`Unknown class: ${propTypeRawWithoutPfx}`);
          }
          if (unnamedNamespace.length > 0) {
            unnamedNamespace += '\n';
          }
          let selfArg = '';
          if (!propStatic) {
            if (propertyJson.setterConst) {
              selfArg = `const ${nameCxx}& self, `;
            } else {
              selfArg = `${nameCxx}& self, `;
            }
          }
          unnamedNamespace += `void ${setterNameCxx}(${selfArg}const std::shared_ptr<psm::${util.getLangClassName(propTypeInterfaceJson, util.CXX)}>& ${setterArgName})\n`;
          unnamedNamespace += `{\n`;
          unnamedNamespace += `    if (!${setterArgName}) {\n`;
          unnamedNamespace += `        assert(!"${nameCxx}::${setterNameCxx}(): ${setterArgName} is null");\n`;
          unnamedNamespace += `        return;\n`;
          unnamedNamespace += `    }\n`;
          if (propStatic) {
            unnamedNamespace += `    psm::${nameCxx}::${setterNameCxx}(*${setterArgName});\n`;
          } else {
            unnamedNamespace += `    self.${setterNameCxx}(*${setterArgName});\n`;
          }
          unnamedNamespace += `}\n`;
          includesFirst.add('#include <cassert>');
        }
        if (propStatic) {
          code += `\n        .class_function("__${setterName}(${setterArgName})", &${funcName})`;
        } else {
          code += `\n        .function("__${setterName}(${setterArgName})", &${funcName})`;
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

function generateJsSource(enums, interfaces, interfaceName, interfaceJson) {
  const name = util.getLangClassName(interfaceJson, util.JS);
  const aliases = util.getLangAliases(interfaceJson, util.JS);

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += '\n';
  code += `posemeshModule.${name} = null;\n`;
  for (const alias of aliases) {
    code += `posemeshModule.${alias} = null;\n`;
  }

  let builderFunctionBody = '';

  for (const propertyJson of util.getProperties(interfaceJson)) {
    const propName = util.getPropertyName(propertyJson, util.JS);
    const propStatic = util.getPropertyStatic(propertyJson);
    const propRootObj = `__internalPosemesh.${name}${propStatic ? '' : '.prototype'}`;
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

  if (builderFunctionBody.length > 0) {
    builderFunctionBody += '\n';
  }
  builderFunctionBody += `    posemeshModule.${name} = __internalPosemesh.${name};\n`;
  for (const alias of aliases) {
    builderFunctionBody += `    posemeshModule.${alias} = __internalPosemesh.${name};\n`;
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

function generateTransformTsDefScript(enums, interfaces, interfaceName, interfaceJson) {
  // TODO: TEMP: this needs to be fully reworked !!!
  const name = util.getLangClassName(interfaceJson, util.JS);
  const fixFuncName = `fix${name}`;
  const isSpecialPosemeshClass = name === 'Posemesh'; // TODO: TEMP: remove, this class is no longer special !!!
  const static = util.getClassStatic(interfaceJson);
  const copyable = util.getClassCopyable(interfaceJson);

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += `\n`;
  code += `function ${fixFuncName}(content, newLine, tab) {\n`;

  code += `    // Find member area\n`;
  code += `    const memberAreaMatches = content.match(/export interface ${name} \\{(.|\\r|\\n)*?\\}/gm);\n`;
  code += `    if (!Array.isArray(memberAreaMatches) || memberAreaMatches.length === 0) {\n`;
  code += `        throw new Error('Member area for \\'${name}\\' not found.');\n`;
  code += `    }\n`;
  code += `    if (memberAreaMatches.length > 1) {\n`;
  code += `        throw new Error('Multiple member areas for \\'${name}\\' found.');\n`;
  code += `    }\n`;
  code += `    const memberArea = memberAreaMatches[0];\n`;
  code += `\n`;
  code += `    // Find static area\n`;
  code += `    const staticAreaMatches = content.match(/${name}: \\{(.|\\r|\\n)*?\\};/gm);\n`;
  code += `    if (!Array.isArray(staticAreaMatches) || staticAreaMatches.length === 0) {\n`;
  code += `        throw new Error('Static area for \\'${name}\\' not found.');\n`;
  code += `    }\n`;
  code += `    if (staticAreaMatches.length > 1) {\n`;
  code += `        throw new Error('Multiple static areas for \\'${name}\\' found.');\n`;
  code += `    }\n`;
  code += `    const staticArea = staticAreaMatches[0];\n`;

  if (isSpecialPosemeshClass || static) {
    code += `\n`;
    code += `    // Remove member area\n`;
    code += `    if (content.include(\`\${newLine}\${newLine}\${memberArea}\`)) {\n`;
    code += `        content = content.replace(\`\${newLine}\${newLine}\${memberArea}\`, '');\n`;
    code += `    } else if (content.include(\`\${memberArea}\${newLine}\${newLine}\`)) {\n`;
    code += `        content = content.replace(\`\${memberArea}\${newLine}\${newLine}\`, '');\n`;
    code += `    } else {\n`;
    code += `        throw new Error('Member area for \\'${name}\\' could not be removed.');\n`;
    code += `    }\n`;
  }

  if (isSpecialPosemeshClass) {
    code += `\n`;
    code += `    // Remove static area\n`;
    code += `    if (content.include(\`\${newLine}\${staticArea}\`)) {\n`;
    code += `        content = content.replace(\`\${newLine}\${staticArea}\`, '');\n`;
    code += `    } else if (content.include(\`\${staticArea}\${newLine}\`)) {\n`;
    code += `        content = content.replace(\`\${staticArea}\${newLine}\`, '');\n`;
    code += `    } else {\n`;
    code += `        throw new Error('Static area for \\'${name}\\' could not be removed.');\n`;
    code += `    }\n`;
  }

  code += `\n`;
  code += `    return content;\n`;

  code += `}\n`;
  code += `\n`;
  code += `module.exports = ${fixFuncName};\n`;
  return code;
}

function generateInterfaceJS(enums, interfaces, interfaceName, interfaceJson) {
  const cppSourceFilePath = path.resolve(__dirname, '..', 'platform', 'Web', 'src', `${interfaceName}.cpp`);
  const jsSourceFilePath = path.resolve(__dirname, '..', 'platform', 'Web', `${interfaceName}.js`);
  const transformTsDefScriptFilePath = path.resolve(__dirname, '..', 'platform', 'Web', `transform-typescript-definition-${interfaceName}.js`);

  let cppSourceCode = generateCppSource(enums, interfaces, interfaceName, interfaceJson);
  let jsSourceCode = generateJsSource(enums, interfaces, interfaceName, interfaceJson);
  let transformTsDefScriptCode = generateTransformTsDefScript(enums, interfaces, interfaceName, interfaceJson);

  util.writeFileContentIfDifferent(cppSourceFilePath, cppSourceCode);
  util.writeFileContentIfDifferent(jsSourceFilePath, jsSourceCode);
  util.writeFileContentIfDifferent(transformTsDefScriptFilePath, transformTsDefScriptCode);
}

module.exports = generateInterfaceJS;
