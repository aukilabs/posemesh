const fs = require('fs');
const path = require('path');
const util = require('./util');

function generateHeader(interfaceName, interfaceJson) {
  const name = util.getLangClassName(interfaceJson, util.C);
  const nameWithoutTSuffix = name.substring(0, name.length - 2);
  const nameCxx = util.getLangClassName(interfaceJson, util.CXX);
  const headerGuardName = util.getHeaderGuardName(interfaceJson);
  const headerGuard = `__POSEMESH_C_${headerGuardName}_H__`;
  const static = util.getClassStatic(interfaceJson);
  const copyable = util.getClassCopyable(interfaceJson);

  let includesFirst = new Set([]), includesSecond = new Set(['#include "API.hpp"']);

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += `\n`;
  code += `#ifndef ${headerGuard}\n`;
  code += `#define ${headerGuard}\n`;
  code += '%INCLUDES%\n';
  code += '#if defined(__cplusplus)\n';
  code += `namespace psm {\n`;
  code += `class ${nameCxx};\n`;
  code += `}\n`;
  code += `typedef psm::${nameCxx} ${name};\n`;
  code += '#else\n';
  code += `typedef struct ${nameWithoutTSuffix} ${name};\n`;
  code += '#endif\n';
  for (const alias of util.getLangAliases(interfaceJson, util.C)) {
    code += `typedef ${name} ${alias};\n`;
  }

  code += `\n`;
  code += '#if defined(__cplusplus)\n';
  code += 'extern "C" {\n';
  code += '#endif\n';

  let publicCtors = '', publicOperators = '', publicMethods = '', publicFuncs = '';

  const parameterlessConstructor = util.getClassParameterlessConstructor(interfaceJson);
  const pCtorDefinition = util.getConstructorDefinition(parameterlessConstructor);
  const pCtorVisibility = util.getConstructorVisibility(parameterlessConstructor);
  if (!static && pCtorDefinition !== util.ConstructorDefinition.deleted && pCtorVisibility === util.Visibility.public) {
    publicCtors += `${name}* PSM_API ${nameWithoutTSuffix}_create();\n`;
  }

  const copyConstructor = util.getClassCopyConstructor(interfaceJson);
  const cCtorDefinition = util.getConstructorDefinition(copyConstructor);
  const cCtorVisibility = util.getConstructorVisibility(copyConstructor);
  if (!static && copyable && cCtorDefinition !== util.ConstructorDefinition.deleted && cCtorVisibility === util.Visibility.public) {
    const mainArgName = util.getCopyOrMoveConstructorMainArgName(copyConstructor, util.C);
    publicCtors += `${name}* PSM_API ${nameWithoutTSuffix}_duplicate(const ${name}* ${mainArgName});\n`;
  }

  if (!static) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    publicCtors += `void PSM_API ${nameWithoutTSuffix}_destroy(${name}* ${mainArgName});\n`;
  }

  const equalityOperator = interfaceJson.equalityOperator;
  if (equalityOperator.defined) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    publicOperators += `uint8_t PSM_API ${nameWithoutTSuffix}_equals(const ${name}* ${mainArgName}, const ${name}* other_${mainArgName});\n`;
    includesFirst.add('#include <stdint.h>');
  }

  const hashOperator = interfaceJson.hashOperator;
  if (hashOperator.defined) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    publicOperators += `size_t PSM_API ${nameWithoutTSuffix}_hash(const ${name}* ${mainArgName});\n`;
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
      const getter = `${getterType} PSM_API ${nameWithoutTSuffix}_${getterName}(${propStatic ? `` : `${getterConstPfx}${name}* ${mainArgName}`});\n`;
      const getterVisibility = util.getPropertyGetterVisibility(propertyJson);
      if (getterVisibility === util.Visibility.public) {
        shouldAddIncludes = true;
        if (propStatic) {
          publicFuncs += getter;
        } else {
          publicMethods += getter;
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
        } else {
          publicMethods += setter;
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

  code += `\n`;
  code += '#if defined(__cplusplus)\n';
  code += '}\n';
  code += '#endif\n';

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
  let includesFirst = new Set([`#include <Posemesh/C/${interfaceName}.h>`]), includesSecond = new Set([]);

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += '%INCLUDES%\n';

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

  fs.writeFileSync(headerFilePath, headerCode, 'utf8');
  fs.writeFileSync(sourceFilePath, sourceCode, 'utf8');
}

module.exports = generateInterfaceC;
