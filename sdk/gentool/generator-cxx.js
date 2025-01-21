const fs = require('fs');
const path = require('path');
const util = require('./util');

function generateHeader(interfaceName, interfaceJson) {
  const name = util.getLangClassName(interfaceJson, util.CXX);
  const aliases = util.getLangAliases(interfaceJson, util.CXX);
  const headerGuardName = util.getHeaderGuardName(interfaceJson);
  const headerGuard = `__POSEMESH_${headerGuardName}_HPP__`;

  let includesFirst = new Set([]), includesSecond = new Set(['#include "API.hpp"']);

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += '\n';
  code += `#ifndef ${headerGuard}\n`;
  code += `#define ${headerGuard}\n`;
  code += '%INCLUDES%\n';
  code += 'namespace psm {\n';
  code += '\n';
  code += `class ${name} {\n`;

  let publicMethods = '', publicFuncs = '', publicMembVars = '', publicStatVars = '';
  let protectedMethods = '', protectedFuncs = '', protectedMembVars = '', protectedStatVars = '';
  let privateMethods = '', privateFuncs = '', privateMembVars = '', privateStatVars = '';

  publicMethods += `    PSM_API ${name}(const ${name}& source);\n`;
  publicMethods += `    PSM_API ${name}(${name}&& source);\n`;
  publicMethods += `    PSM_API ~${name}();\n`;
  publicMethods += '\n';

  for (const propertyJson of util.getProperties(interfaceJson)) {
    const propName = util.getPropertyName(propertyJson, util.CXX);
    const propType = util.getPropertyType(propertyJson, util.CXX);
    const propStatic = util.getPropertyStatic(propertyJson);
    const propStaticPfx = propStatic ? 'static ' : '';
    if (util.getPropertyHasMemberVar(propertyJson)) {
      const m = `    ${propStaticPfx}${propType} ${propName};\n`;
      if (propStatic) {
        privateStatVars += m;
      } else {
        privateMembVars += m;
      }
    }
    if (propertyJson.hasGetter) {
      const getterConstExt = propertyJson.getterConst ? ' const' : '';
      const getterNoexceptExt = propertyJson.getterNoexcept ? ' noexcept' : '';
      const getterName = util.getPropertyGetterName(propertyJson, util.CXX);
      const getterType = util.getPropertyTypeForGetter(propertyJson, util.CXX);
      const getterMode = util.getPropertyGetterMode(propertyJson);
      const getterVirtualPfx = getterMode !== util.MethodMode.regular ? 'virtual ' : '';
      const getterVirtualExt = getterMode === util.MethodMode.pureVirtual ? ' = 0' : (getterMode === util.MethodMode.override ? ' override' : '');
      const getter = `    ${propStaticPfx}${getterVirtualPfx}${getterType} PSM_API ${getterName}()${getterConstExt}${getterNoexceptExt}${getterVirtualExt};\n`;
      const getterVisibility = util.getPropertyGetterVisibility(propertyJson);
      switch (getterVisibility) {
        case util.Visibility.public:
          if (propStatic) {
            publicFuncs += getter;
          } else {
            publicMethods += getter;
          }
          break;
        case util.Visibility.protected:
          if (propStatic) {
            protectedFuncs += getter;
          } else {
            protectedMethods += getter;
          }
          break;
        case util.Visibility.private:
          if (propStatic) {
            privateFuncs += getter;
          } else {
            privateMethods += getter;
          }
          break;
        default:
          throw new Error('Unhandled C++ getter visibility.');
      }
    }
    if (propertyJson.hasSetter) {
      const setterConstExt = propertyJson.setterConst ? ' const' : '';
      const setterNoexceptExt = propertyJson.setterNoexcept ? ' noexcept' : '';
      const setterName = util.getPropertySetterName(propertyJson, util.CXX);
      const setterType = util.getPropertyTypeForSetter(propertyJson, util.CXX);
      const setterArgName = util.getPropertySetterArgName(propertyJson, util.CXX);
      const setterMode = util.getPropertySetterMode(propertyJson);
      const setterVirtualPfx = setterMode !== util.MethodMode.regular ? 'virtual ' : '';
      const setterVirtualExt = setterMode === util.MethodMode.pureVirtual ? ' = 0' : (setterMode === util.MethodMode.override ? ' override' : '');
      const setter = `    ${propStaticPfx}${setterVirtualPfx}void PSM_API ${setterName}(${setterType} ${setterArgName})${setterConstExt}${setterNoexceptExt}${setterVirtualExt};\n`;
      const setterVisibility = util.getPropertySetterVisibility(propertyJson);
      switch (setterVisibility) {
        case util.Visibility.public:
          if (propStatic) {
            publicFuncs += setter;
          } else {
            publicMethods += setter;
          }
          break;
        case util.Visibility.protected:
          if (propStatic) {
            protectedFuncs += setter;
          } else {
            protectedMethods += setter;
          }
          break;
        case util.Visibility.private:
          if (propStatic) {
            privateFuncs += setter;
          } else {
            privateMethods += setter;
          }
          break;
        default:
          throw new Error('Unhandled C++ setter visibility.');
      }
    }
  }

  let public = publicMethods, protected = protectedMethods, private = privateMethods;

  if (publicFuncs.length > 0) {
    if (public.length > 0) {
      public += '\n';
    }
    public += publicFuncs;
  }
  if (publicMembVars.length > 0) {
    if (public.length > 0) {
      public += '\n';
    }
    public += publicMembVars;
  }
  if (publicStatVars.length > 0) {
    if (public.length > 0) {
      public += '\n';
    }
    public += publicStatVars;
  }

  if (protectedFuncs.length > 0) {
    if (protected.length > 0) {
      protected += '\n';
    }
    protected += protectedFuncs;
  }
  if (protectedMembVars.length > 0) {
    if (protected.length > 0) {
      protected += '\n';
    }
    protected += protectedMembVars;
  }
  if (protectedStatVars.length > 0) {
    if (protected.length > 0) {
      protected += '\n';
    }
    protected += protectedStatVars;
  }

  if (privateFuncs.length > 0) {
    if (private.length > 0) {
      private += '\n';
    }
    private += privateFuncs;
  }
  if (privateMembVars.length > 0) {
    if (private.length > 0) {
      private += '\n';
    }
    private += privateMembVars;
  }
  if (privateStatVars.length > 0) {
    if (private.length > 0) {
      private += '\n';
    }
    private += privateStatVars;
  }

  if (public.length > 0) {
    code += `public:\n${public}`;
  }
  if (protected.length > 0) {
    if (public.length > 0) {
      code += '\n';
    }
    code += `protected:\n${protected}`;
  }
  if (private.length > 0) {
    if (public.length > 0 || protected.length > 0) {
      code += '\n';
    }
    code += `private:\n${private}`;
  }

  code += '};\n';
  for (const alias of aliases) {
    code += `using ${alias} = ${name};\n`;
  }
  code += '\n';
  code += '}\n';
  code += '\n';
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
  const name = util.getLangClassName(interfaceJson, util.CXX);

  let includesFirst = new Set([`#include <Posemesh/${interfaceName}.hpp>`]), includesSecond = new Set([]);

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += '%INCLUDES%\n';
  code += 'namespace psm {\n';
  code += '\n';

  let publicMethods = '', publicFuncs = '', publicStatVars = '';
  let protectedMethods = '', protectedFuncs = '', protectedStatVars = '';
  let privateMethods = '', privateFuncs = '', privateStatVars = '';

  for (const propertyJson of util.getProperties(interfaceJson)) {
    const propName = util.getPropertyName(propertyJson, util.CXX);
    const propType = util.getPropertyType(propertyJson, util.CXX);
    const propStatic = util.getPropertyStatic(propertyJson);
    if (propStatic) {
      privateStatVars += `${propType} ${name}::${propName}{};\n`;
    }
    const hasGetter = propertyJson.hasGetter;
    const getterCustom = propertyJson.getterCustom;
    if (hasGetter && !getterCustom) {
      const getterConstExt = propertyJson.getterConst ? ' const' : '';
      const getterNoexceptExt = propertyJson.getterNoexcept ? ' noexcept' : '';
      const getterName = util.getPropertyGetterName(propertyJson, util.CXX);
      const getterType = util.getPropertyTypeForGetter(propertyJson, util.CXX);
      let getter = `${getterType} ${name}::${getterName}()${getterConstExt}${getterNoexceptExt}\n`;
      getter += '{\n';
      getter += `    return ${propName};\n`;
      getter += '}\n';
      const getterVisibility = util.getPropertyGetterVisibility(propertyJson);
      switch (getterVisibility) {
        case util.Visibility.public:
          if (propStatic) {
            if (publicFuncs.length > 0) { publicFuncs += '\n'; }
            publicFuncs += getter;
          } else {
            if (publicMethods.length > 0) { publicMethods += '\n'; }
            publicMethods += getter;
          }
          break;
        case util.Visibility.protected:
          if (propStatic) {
            if (protectedFuncs.length > 0) { protectedFuncs += '\n'; }
            protectedFuncs += getter;
          } else {
            if (protectedMethods.length > 0) { protectedMethods += '\n'; }
            protectedMethods += getter;
          }
          break;
        case util.Visibility.private:
          if (propStatic) {
            if (privateFuncs.length > 0) { privateFuncs += '\n'; }
            privateFuncs += getter;
          } else {
            if (privateMethods.length > 0) { privateMethods += '\n'; }
            privateMethods += getter;
          }
          break;
        default:
          throw new Error('Unhandled C++ getter visibility.');
      }
    }
    const hasSetter = propertyJson.hasSetter;
    const setterCustom = propertyJson.setterCustom;
    if (hasSetter && !setterCustom) {
      const setterConstExt = propertyJson.setterConst ? ' const' : '';
      const setterNoexceptExt = propertyJson.setterNoexcept ? ' noexcept' : '';
      const setterName = util.getPropertySetterName(propertyJson, util.CXX);
      const setterType = util.getPropertyTypeForSetter(propertyJson, util.CXX);
      const setterArgName = util.getPropertySetterArgName(propertyJson, util.CXX);
      let setter = `void ${name}::${setterName}(${setterType} ${setterArgName})${setterConstExt}${setterNoexceptExt}\n`;
      setter += '{\n';
      setter += `    ${propName} = ${setterArgName};\n`;
      setter += '}\n';
      const setterVisibility = util.getPropertySetterVisibility(propertyJson);
      switch (setterVisibility) {
        case util.Visibility.public:
          if (propStatic) {
            if (publicFuncs.length > 0) { publicFuncs += '\n'; }
            publicFuncs += setter;
          } else {
            if (publicMethods.length > 0) { publicMethods += '\n'; }
            publicMethods += setter;
          }
          break;
        case util.Visibility.protected:
          if (propStatic) {
            if (protectedFuncs.length > 0) { protectedFuncs += '\n'; }
            protectedFuncs += setter;
          } else {
            if (protectedMethods.length > 0) { protectedMethods += '\n'; }
            protectedMethods += setter;
          }
          break;
        case util.Visibility.private:
          if (propStatic) {
            if (privateFuncs.length > 0) { privateFuncs += '\n'; }
            privateFuncs += setter;
          } else {
            if (privateMethods.length > 0) { privateMethods += '\n'; }
            privateMethods += setter;
          }
          break;
        default:
          throw new Error('Unhandled C++ setter visibility.');
      }
    }
  }

  let public = publicMethods, protected = protectedMethods, private = privateMethods;

  if (publicFuncs.length > 0) {
    if (public.length > 0) {
      public += '\n';
    }
    public += publicFuncs;
  }
  if (publicStatVars.length > 0) {
    if (public.length > 0) {
      public += '\n';
    }
    public += publicStatVars;
  }

  if (protectedFuncs.length > 0) {
    if (protected.length > 0) {
      protected += '\n';
    }
    protected += protectedFuncs;
  }
  if (protectedStatVars.length > 0) {
    if (protected.length > 0) {
      protected += '\n';
    }
    protected += protectedStatVars;
  }

  if (privateFuncs.length > 0) {
    if (private.length > 0) {
      private += '\n';
    }
    private += privateFuncs;
  }
  if (privateStatVars.length > 0) {
    if (private.length > 0) {
      private += '\n';
    }
    private += privateStatVars;
  }

  let namespaceBodyNeedsAdditionalNewLine = false;
  if (public.length > 0) {
    namespaceBodyNeedsAdditionalNewLine = true;
    code += public;
  }
  if (protected.length > 0) {
    namespaceBodyNeedsAdditionalNewLine = true;
    if (public.length > 0) {
      code += '\n';
    }
    code += protected;
  }
  if (private.length > 0) {
    namespaceBodyNeedsAdditionalNewLine = true;
    if (public.length > 0 || protected.length > 0) {
      code += '\n';
    }
    code += private;
  }

  if (namespaceBodyNeedsAdditionalNewLine) { code += '\n'; }
  code += '}\n';

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

function generateInterfaceCXX(interfaceName, interfaceJson) {
  const headerFilePath = path.resolve(__dirname, '..', 'include', 'Posemesh', `${interfaceName}.hpp`);
  const sourceFilePath = path.resolve(__dirname, '..', 'src', `${interfaceName}.gen.cpp`);

  let headerCode = generateHeader(interfaceName, interfaceJson);
  let sourceCode = generateSource(interfaceName, interfaceJson);

  fs.writeFileSync(headerFilePath, headerCode, 'utf8');
  fs.writeFileSync(sourceFilePath, sourceCode, 'utf8');
}

module.exports = generateInterfaceCXX;
