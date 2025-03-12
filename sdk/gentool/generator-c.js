const path = require('path');
const util = require('./util');

function generateHeader(interfaceName, interfaceJson) {
  const name = util.getLangClassName(interfaceJson, util.C);
  const nameWithoutTSuffix = name.substring(0, name.length - 2);
  const nameRefWithoutTSuffix = `${nameWithoutTSuffix}_ref`;
  const nameRef = `${nameRefWithoutTSuffix}_t`;
  const nameCxx = util.getLangClassName(interfaceJson, util.CXX);
  const headerGuardName = util.getHeaderGuardName(interfaceJson);
  const headerGuard = `__POSEMESH_C_${headerGuardName}_H__`;
  const static = util.getClassStatic(interfaceJson);
  const copyable = util.getClassCopyable(interfaceJson);

  let includesFirst = new Set([]), includesSecond = new Set(['#include "API.h"']);

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += `\n`;
  code += `#ifndef ${headerGuard}\n`;
  code += `#define ${headerGuard}\n`;
  code += '%INCLUDES%\n';
  code += '#if defined(__cplusplus)\n';
  if (!static) {
    code += `#include <memory>\n`;
  }
  code += `namespace psm {\n`;
  code += `class ${nameCxx};\n`;
  code += `}\n`;
  code += `typedef psm::${nameCxx} ${name};\n`;
  if (!static) {
    code += `typedef std::shared_ptr<${name}> ${nameRef};\n`;
  }
  code += '#else\n';
  code += `typedef struct ${nameWithoutTSuffix} ${name};\n`;
  if (!static) {
    code += `typedef struct ${nameRefWithoutTSuffix} ${nameRef};\n`;
  }
  code += '#endif\n';
  for (const alias of util.getLangAliases(interfaceJson, util.C)) {
    code += `typedef ${name} ${alias};\n`;
  }
  if (!static) {
    for (const alias of util.getLangAliases(interfaceJson, util.C)) {
      code += `typedef ${nameRef} ${alias.substring(0, alias.length - 2)}_ref_t;\n`;
    }
  }

  code += `\n`;
  code += '#if defined(__cplusplus)\n';
  code += 'extern "C" {\n';
  code += '#endif\n';

  let publicCtors = '', publicOperators = '', publicMethods = '', publicFuncs = '';
  let funcAliases = [];

  const parameterlessConstructor = util.getClassParameterlessConstructor(interfaceJson);
  const pCtorDefinition = util.getConstructorDefinition(parameterlessConstructor);
  const pCtorVisibility = util.getConstructorVisibility(parameterlessConstructor);
  if (!static && pCtorDefinition !== util.ConstructorDefinition.deleted && pCtorVisibility === util.Visibility.public) {
    publicCtors += `${name}* PSM_API ${nameWithoutTSuffix}_create();\n`;
    funcAliases.push({
      name: `${nameWithoutTSuffix}_create`,
      args: []
    });
  }

  const copyConstructor = util.getClassCopyConstructor(interfaceJson);
  const cCtorDefinition = util.getConstructorDefinition(copyConstructor);
  const cCtorVisibility = util.getConstructorVisibility(copyConstructor);
  if (!static && copyable && cCtorDefinition !== util.ConstructorDefinition.deleted && cCtorVisibility === util.Visibility.public) {
    const mainArgName = util.getCopyOrMoveConstructorMainArgName(copyConstructor, util.C);
    publicCtors += `${name}* PSM_API ${nameWithoutTSuffix}_duplicate(const ${name}* ${mainArgName});\n`;
    funcAliases.push({
      name: `${nameWithoutTSuffix}_duplicate`,
      args: [mainArgName]
    });
  }

  if (!static) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    publicCtors += `void PSM_API ${nameWithoutTSuffix}_destroy(${name}* ${mainArgName});\n`;
    funcAliases.push({
      name: `${nameWithoutTSuffix}_destroy`,
      args: [mainArgName]
    });
  }

  const equalityOperator = interfaceJson.equalityOperator;
  if (equalityOperator.defined) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    publicOperators += `uint8_t PSM_API ${nameWithoutTSuffix}_equals(const ${name}* ${mainArgName}, const ${name}* other_${mainArgName});\n`;
    funcAliases.push({
      name: `${nameWithoutTSuffix}_equals`,
      args: [mainArgName, `other_${mainArgName}`]
    });
    includesFirst.add('#include <stdint.h>');
  }

  const hashOperator = interfaceJson.hashOperator;
  if (hashOperator.defined) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    publicOperators += `size_t PSM_API ${nameWithoutTSuffix}_hash(const ${name}* ${mainArgName});\n`;
    funcAliases.push({
      name: `${nameWithoutTSuffix}_hash`,
      args: [mainArgName]
    });
    includesFirst.add('#include <stddef.h>');
  }

  for (const propertyJson of util.getProperties(interfaceJson)) {
    let shouldAddIncludes = false;
    const propStatic = util.getPropertyStatic(propertyJson);
    if (propertyJson.hasGetter) {
      const getterName = util.getPropertyGetterName(propertyJson, util.C);
      const getterType = util.getPropertyTypeForGetter(propertyJson, util.C);
      const getterConstPfx = propertyJson.getterConst ? 'const ' : '';
      const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
      const setterArgName = util.getPropertySetterArgName(propertyJson, util.C);
      const getter = `${getterType} PSM_API ${nameWithoutTSuffix}_${getterName}(${propStatic ? `` : `${getterConstPfx}${name}* ${mainArgName}`});\n`;
      const getterFree = `void PSM_API ${nameWithoutTSuffix}_${getterName}_free(${getterType} ${setterArgName});\n`;
      const getterVisibility = util.getPropertyGetterVisibility(propertyJson);
      if (getterVisibility === util.Visibility.public) {
        shouldAddIncludes = true;
        if (propStatic) {
          publicFuncs += getter;
          funcAliases.push({
            name: `${nameWithoutTSuffix}_${getterName}`,
            args: []
          });
          if (propertyJson.type === 'string') {
            publicFuncs += getterFree;
            funcAliases.push({
              name: `${nameWithoutTSuffix}_${getterName}_free`,
              args: [setterArgName]
            });
          }
        } else {
          publicMethods += getter;
          funcAliases.push({
            name: `${nameWithoutTSuffix}_${getterName}`,
            args: [mainArgName]
          });
          if (propertyJson.type === 'string') {
            publicMethods += getterFree;
            funcAliases.push({
              name: `${nameWithoutTSuffix}_${getterName}_free`,
              args: [setterArgName]
            });
          }
        }
      }
    }
    if (propertyJson.hasSetter) {
      const setterName = util.getPropertySetterName(propertyJson, util.C);
      const setterType = util.getPropertyTypeForSetter(propertyJson, util.C);
      const setterConstPfx = propertyJson.setterConst ? 'const ' : '';
      const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
      const setterArgName = util.getPropertySetterArgName(propertyJson, util.C);
      const setter = `void PSM_API ${nameWithoutTSuffix}_${setterName}(${propStatic ? `` : `${setterConstPfx}${name}* ${mainArgName}, `}${setterType} ${setterArgName});\n`;
      const setterVisibility = util.getPropertySetterVisibility(propertyJson);
      if (setterVisibility === util.Visibility.public) {
        shouldAddIncludes = true;
        if (propStatic) {
          publicFuncs += setter;
          funcAliases.push({
            name: `${nameWithoutTSuffix}_${setterName}`,
            args: [setterArgName]
          });
        } else {
          publicMethods += setter;
          funcAliases.push({
            name: `${nameWithoutTSuffix}_${setterName}`,
            args: [mainArgName, setterArgName]
          });
        }
      }
    }
    if (shouldAddIncludes) {
      const propTypeRaw = propertyJson.type;
      if (util.isIntType(propTypeRaw) || propTypeRaw === 'boolean') {
        includesFirst.add('#include <stdint.h>');
      }
    }
  }

  let public = publicCtors;
  if (publicOperators.length > 0) {
    if (public.length > 0) {
      public += '\n';
    }
    public += publicOperators;
  }
  if (publicMethods.length > 0) {
    if (public.length > 0) {
      public += '\n';
    }
    public += publicMethods;
  }
  if (publicFuncs.length > 0) {
    if (public.length > 0) {
      public += '\n';
    }
    public += publicFuncs;
  }

  if (public.length > 0) {
    code += '\n';
    code += public;
  }

  if (!static) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    code += `\n`;
    code += `${nameRef}* PSM_API ${nameRefWithoutTSuffix}_make(${name}* ${mainArgName});\n`;
    funcAliases.push({
      name: `${nameRefWithoutTSuffix}_make`,
      args: [mainArgName]
    });
    code += `${nameRef}* PSM_API ${nameRefWithoutTSuffix}_clone(const ${nameRef}* ${mainArgName}_ref);\n`;
    funcAliases.push({
      name: `${nameRefWithoutTSuffix}_clone`,
      args: [`${mainArgName}_ref`]
    });
    code += `${name}* PSM_API ${nameRefWithoutTSuffix}_get(const ${nameRef}* ${mainArgName}_ref);\n`;
    funcAliases.push({
      name: `${nameRefWithoutTSuffix}_get`,
      args: [`${mainArgName}_ref`]
    });
    code += `void PSM_API ${nameRefWithoutTSuffix}_delete(${nameRef}* ${mainArgName}_ref);\n`;
    funcAliases.push({
      name: `${nameRefWithoutTSuffix}_delete`,
      args: [`${mainArgName}_ref`]
    });
  }

  code += `\n`;
  code += '#if defined(__cplusplus)\n';
  code += '}\n';
  code += '#endif\n';

  if (interfaceJson['c.generateFuncAliasDefines'] && funcAliases.length > 0) {
    for (const alias of util.getLangAliases(interfaceJson, util.C)) {
      const aliasWithoutTSuffix = alias.substring(0, alias.length - 2);
      code += '\n';
      for (const funcAlias of funcAliases) {
        let args = '';
        let argsSent = '';
        for (const arg of funcAlias.args) {
          if (args.length > 0) {
            args += ', ';
            argsSent += ', ';
          }
          args += `_${arg}`;
          argsSent += `(_${arg})`;
        }
        code += `#define ${aliasWithoutTSuffix}${funcAlias.name.substring(nameWithoutTSuffix.length)}(${args}) (${funcAlias.name}(${argsSent}))\n`;
      }
    }
  }

  code += `\n`;
  code += `#endif // ${headerGuard}\n`;

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

  return code;
}

function generateSource(interfaceName, interfaceJson) {
  const name = util.getLangClassName(interfaceJson, util.C);
  const nameWithoutTSuffix = name.substring(0, name.length - 2);
  const nameRefWithoutTSuffix = `${nameWithoutTSuffix}_ref`;
  const nameRef = `${nameRefWithoutTSuffix}_t`;
  const nameCxx = util.getLangClassName(interfaceJson, util.CXX);
  const static = util.getClassStatic(interfaceJson);
  const copyable = util.getClassCopyable(interfaceJson);

  let includesFirst = new Set([`#include <Posemesh/C/${interfaceName}.h>`, `#include <Posemesh/${interfaceName}.hpp>`]), includesSecond = new Set([]);

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += '%INCLUDES%';

  let publicCtors = '', publicOperators = '', publicMethods = '', publicFuncs = '';
  let privateCtors = '', privateOperators = '', privateMethods = '', privateFuncs = '';

  const parameterlessConstructor = util.getClassParameterlessConstructor(interfaceJson);
  const pCtorDefinition = util.getConstructorDefinition(parameterlessConstructor);
  const pCtorVisibility = util.getConstructorVisibility(parameterlessConstructor);
  if (!static && pCtorDefinition !== util.ConstructorDefinition.deleted) {
    let pCtor = `${name}* ${nameWithoutTSuffix}_create()\n`;
    pCtor += `{\n`;
    pCtor += `    return new (std::nothrow) psm::${nameCxx};\n`;
    pCtor += `}\n`;
    if (pCtorVisibility === util.Visibility.public) {
      if (publicCtors.length > 0) {
        publicCtors += '\n';
      }
      publicCtors += pCtor;
    } else {
      if (privateCtors.length > 0) {
        privateCtors += '\n';
      }
      privateCtors += pCtor;
    }

    includesFirst.add('#include <new>');
  }

  const copyConstructor = util.getClassCopyConstructor(interfaceJson);
  const cCtorDefinition = util.getConstructorDefinition(copyConstructor);
  const cCtorVisibility = util.getConstructorVisibility(copyConstructor);
  if (!static && copyable && cCtorDefinition !== util.ConstructorDefinition.deleted) {
    const mainArgName = util.getCopyOrMoveConstructorMainArgName(copyConstructor, util.C);
    let cCtor = `${name}* ${nameWithoutTSuffix}_duplicate(const ${name}* ${mainArgName})\n`;
    cCtor += `{\n`;
    cCtor += `    if (!${mainArgName}) {\n`;
    cCtor += `        assert(!"${nameWithoutTSuffix}_duplicate(): ${mainArgName} is null");\n`;
    cCtor += `        return nullptr;\n`;
    cCtor += `    }\n`;
    cCtor += `    return new (std::nothrow) psm::${nameCxx}(*${mainArgName});\n`;
    cCtor += `}\n`;
    if (cCtorVisibility === util.Visibility.public) {
      if (publicCtors.length > 0) {
        publicCtors += '\n';
      }
      publicCtors += cCtor;
    } else {
      if (privateCtors.length > 0) {
        privateCtors += '\n';
      }
      privateCtors += cCtor;
    }

    includesFirst.add('#include <cassert>');
    includesFirst.add('#include <new>');
  }

  if (!static) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    let dtor = `void ${nameWithoutTSuffix}_destroy(${name}* ${mainArgName})\n`;
    dtor += `{\n`;
    dtor += `    delete ${mainArgName};\n`;
    dtor += `}\n`;
    if (publicCtors.length > 0) {
      publicCtors += '\n';
    }
    publicCtors += dtor;
  }

  const equalityOperator = interfaceJson.equalityOperator;
  if (equalityOperator.defined) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    let eqOp = `uint8_t ${nameWithoutTSuffix}_equals(const ${name}* ${mainArgName}, const ${name}* other_${mainArgName})\n`;
    eqOp += `{\n`;
    eqOp += `    if (!${mainArgName}) {\n`;
    eqOp += `        assert(!"${nameWithoutTSuffix}_equals(): ${mainArgName} is null");\n`;
    eqOp += `        return 0;\n`;
    eqOp += `    }\n`;
    eqOp += `    if (!other_${mainArgName}) {\n`;
    eqOp += `        assert(!"${nameWithoutTSuffix}_equals(): other_${mainArgName} is null");\n`;
    eqOp += `        return 0;\n`;
    eqOp += `    }\n`;
    eqOp += `    return static_cast<uint8_t>(${mainArgName}->operator==(*other_${mainArgName}));\n`;
    eqOp += `}\n`;

    if (publicOperators.length > 0) {
      publicOperators += '\n';
    }
    publicOperators += eqOp;

    includesFirst.add('#include <cassert>');
  }

  const hashOperator = interfaceJson.hashOperator;
  if (hashOperator.defined) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    let hashOp = `size_t ${nameWithoutTSuffix}_hash(const ${name}* ${mainArgName})\n`;
    hashOp += `{\n`;
    hashOp += `    if (!${mainArgName}) {\n`;
    hashOp += `        assert(!"${nameWithoutTSuffix}_hash(): ${mainArgName} is null");\n`;
    hashOp += `        return 0;\n`;
    hashOp += `    }\n`;
    hashOp += `    return std::hash<psm::${nameCxx}> {}(*${mainArgName});\n`;
    hashOp += `}\n`;

    if (publicOperators.length > 0) {
      publicOperators += '\n';
    }
    publicOperators += hashOp;

    includesFirst.add('#include <cassert>');
  }

  for (const propertyJson of util.getProperties(interfaceJson)) {
    const propTypeRaw = propertyJson.type;
    const propStatic = util.getPropertyStatic(propertyJson);
    if (propertyJson.hasGetter) {
      const getterName = util.getPropertyGetterName(propertyJson, util.C);
      const getterType = util.getPropertyTypeForGetter(propertyJson, util.C);
      const getterConstPfx = propertyJson.getterConst ? 'const ' : '';
      const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
      const setterArgName = util.getPropertySetterArgName(propertyJson, util.C);
      let getter = `${getterType} ${nameWithoutTSuffix}_${getterName}(${propStatic ? `` : `${getterConstPfx}${name}* ${mainArgName}`})\n`;
      getter += `{\n`;
      if (propStatic) {
        if (propTypeRaw === 'boolean') {
          getter += `    return static_cast<uint8_t>(psm::${nameCxx}::${util.getPropertyGetterName(propertyJson, util.CXX)}());\n`;
        } else if (propTypeRaw === 'string') {
          getter += `    const auto ${setterArgName} = psm::${nameCxx}::${util.getPropertyGetterName(propertyJson, util.CXX)}();\n`;
          getter += `    auto* getter_result = new (std::nothrow) char[${setterArgName}.size() + 1];\n`;
          getter += `    if (!getter_result) {\n`;
          getter += `        return nullptr;\n`;
          getter += `    }\n`;
          getter += `    std::memcpy(getter_result, ${setterArgName}.c_str(), ${setterArgName}.size() + 1);\n`;
          getter += `    return getter_result;\n`;
        } else {
          getter += `    return psm::${nameCxx}::${util.getPropertyGetterName(propertyJson, util.CXX)}();\n`;
        }
      } else {
        getter += `    if (!${mainArgName}) {\n`;
        getter += `        assert(!"${nameWithoutTSuffix}_${getterName}(): ${mainArgName} is null");\n`;
        if (propTypeRaw === 'boolean') {
          getter += `        return static_cast<uint8_t>(${util.getTypeImplicitDefaultValue(propTypeRaw)});\n`;
        } else if (propTypeRaw === 'string') {
          getter += `        return nullptr;\n`;
        } else {
          getter += `        return ${util.getTypeImplicitDefaultValue(propTypeRaw)};\n`;
        }
        getter += `    }\n`;
        if (propTypeRaw === 'boolean') {
          getter += `    return static_cast<uint8_t>(${mainArgName}->${util.getPropertyGetterName(propertyJson, util.CXX)}());\n`;
        } else if (propTypeRaw === 'string') {
          getter += `    const auto ${setterArgName} = ${mainArgName}->${util.getPropertyGetterName(propertyJson, util.CXX)}();\n`;
          getter += `    auto* getter_result = new (std::nothrow) char[${setterArgName}.size() + 1];\n`;
          getter += `    if (!getter_result) {\n`;
          getter += `        return nullptr;\n`;
          getter += `    }\n`;
          getter += `    std::memcpy(getter_result, ${setterArgName}.c_str(), ${setterArgName}.size() + 1);\n`;
          getter += `    return getter_result;\n`;
        } else {
          getter += `    return ${mainArgName}->${util.getPropertyGetterName(propertyJson, util.CXX)}();\n`;
        }
      }
      getter += `}\n`;
      if (propTypeRaw === 'string') {
        getter += `\n`;
        getter += `void ${nameWithoutTSuffix}_${getterName}_free(${getterType} ${setterArgName})\n`;
        getter += `{\n`;
        getter += `    delete[] const_cast<char*>(${setterArgName});\n`;
        getter += `}\n`;
      }
      const getterVisibility = util.getPropertyGetterVisibility(propertyJson);
      if (getterVisibility === util.Visibility.public) {
        if (propStatic) {
          if (publicFuncs.length > 0) {
            publicFuncs += '\n';
          }
          publicFuncs += getter;
        } else {
          if (publicMethods.length > 0) {
            publicMethods += '\n';
          }
          publicMethods += getter;
          includesFirst.add('#include <cassert>');
        }
        if (propTypeRaw === 'string') {
          includesFirst.add('#include <cstring>');
          includesFirst.add('#include <new>');
        }
      }
    }
    if (propertyJson.hasSetter) {
      const setterName = util.getPropertySetterName(propertyJson, util.C);
      const setterType = util.getPropertyTypeForSetter(propertyJson, util.C);
      const setterConstPfx = propertyJson.setterConst ? 'const ' : '';
      const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
      const setterArgName = util.getPropertySetterArgName(propertyJson, util.C);
      let setter = `void ${nameWithoutTSuffix}_${setterName}(${propStatic ? `` : `${setterConstPfx}${name}* ${mainArgName}, `}${setterType} ${setterArgName})\n`;
      setter += `{\n`;
      if (propStatic) {
        if (propTypeRaw === 'boolean') {
          setter += `    psm::${nameCxx}::${util.getPropertySetterName(propertyJson, util.CXX)}(static_cast<bool>(${setterArgName}));\n`;
        } else if (propTypeRaw === 'string') {
          setter += `    psm::${nameCxx}::${util.getPropertySetterName(propertyJson, util.CXX)}(${setterArgName} ? std::string { ${setterArgName} } : std::string {});\n`;
        } else {
          setter += `    psm::${nameCxx}::${util.getPropertySetterName(propertyJson, util.CXX)}(${setterArgName});\n`;
        }
      } else {
        setter += `    if (!${mainArgName}) {\n`;
        setter += `        assert(!"${nameWithoutTSuffix}_${setterName}(): ${mainArgName} is null");\n`;
        setter += `        return;\n`;
        setter += `    }\n`;
        if (propTypeRaw === 'boolean') {
          setter += `    ${mainArgName}->${util.getPropertySetterName(propertyJson, util.CXX)}(static_cast<bool>(${setterArgName}));\n`;
        } else if (propTypeRaw === 'string') {
          setter += `    ${mainArgName}->${util.getPropertySetterName(propertyJson, util.CXX)}(${setterArgName} ? std::string { ${setterArgName} } : std::string {});\n`;
        } else {
          setter += `    ${mainArgName}->${util.getPropertySetterName(propertyJson, util.CXX)}(${setterArgName});\n`;
        }
      }
      setter += `}\n`;
      const setterVisibility = util.getPropertySetterVisibility(propertyJson);
      if (setterVisibility === util.Visibility.public) {
        if (propStatic) {
          if (publicFuncs.length > 0) {
            publicFuncs += '\n';
          }
          publicFuncs += setter;
        } else {
          if (publicMethods.length > 0) {
            publicMethods += '\n';
          }
          publicMethods += setter;
          includesFirst.add('#include <cassert>');
        }
      }
    }
  }

  let public = publicCtors;
  if (publicOperators.length > 0) {
    if (public.length > 0) {
      public += '\n';
    }
    public += publicOperators;
  }
  if (publicMethods.length > 0) {
    if (public.length > 0) {
      public += '\n';
    }
    public += publicMethods;
  }
  if (publicFuncs.length > 0) {
    if (public.length > 0) {
      public += '\n';
    }
    public += publicFuncs;
  }

  let private = privateCtors;
  if (privateOperators.length > 0) {
    if (private.length > 0) {
      private += '\n';
    }
    private += privateOperators;
  }
  if (privateMethods.length > 0) {
    if (private.length > 0) {
      private += '\n';
    }
    private += privateMethods;
  }
  if (privateFuncs.length > 0) {
    if (private.length > 0) {
      private += '\n';
    }
    private += privateFuncs;
  }

  if (public.length > 0) {
    code += '\n';
    code += public;
  }
  if (private.length > 0) {
    code += '\n';
    code += private;
  }

  if (!static) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    code += `\n`;
    code += `${nameRef}* ${nameRefWithoutTSuffix}_make(${name}* ${mainArgName})\n`;
    code += `{\n`;
    code += `    return new (std::nothrow) ${nameRef}(${mainArgName}, &${nameWithoutTSuffix}_destroy);\n`;
    code += `}\n`;

    includesFirst.add('#include <new>');

    code += `\n`;
    code += `${nameRef}* ${nameRefWithoutTSuffix}_clone(const ${nameRef}* ${mainArgName}_ref)\n`;
    code += `{\n`;
    code += `    if (!${mainArgName}_ref) {\n`;
    code += `        return nullptr;\n`;
    code += `    }\n`;
    code += `    return new (std::nothrow) ${nameRef}(*${mainArgName}_ref);\n`;
    code += `}\n`;

    includesFirst.add('#include <new>');

    code += `\n`;
    code += `${name}* ${nameRefWithoutTSuffix}_get(const ${nameRef}* ${mainArgName}_ref)\n`;
    code += `{\n`;
    code += `    if (!${mainArgName}_ref) {\n`;
    code += `        return nullptr;\n`;
    code += `    }\n`;
    code += `    return ${mainArgName}_ref->get();\n`;
    code += `}\n`;

    code += `\n`;
    code += `void ${nameRefWithoutTSuffix}_delete(${nameRef}* ${mainArgName}_ref)\n`;
    code += `{\n`;
    code += `    delete ${mainArgName}_ref;\n`;
    code += `}\n`;
  }

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

  return code;
}

function generateInterfaceC(interfaceName, interfaceJson) {
  const headerFilePath = path.resolve(__dirname, '..', 'include', 'Posemesh', 'C', `${interfaceName}.h`);
  const sourceFilePath = path.resolve(__dirname, '..', 'src', 'C', `${interfaceName}.cpp`);

  let headerCode = generateHeader(interfaceName, interfaceJson);
  let sourceCode = generateSource(interfaceName, interfaceJson);

  util.writeFileContentIfDifferent(headerFilePath, headerCode);
  util.writeFileContentIfDifferent(sourceFilePath, sourceCode);
}

module.exports = generateInterfaceC;
