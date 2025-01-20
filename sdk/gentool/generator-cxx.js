const fs = require('fs');
const path = require('path');
const util = require('./util');

function generateHeader(interfaceName, interfaceJson) {
  const name = util.getLangClassName(interfaceJson, util.CXX);
  const aliases = util.getLangAliases(interfaceJson, util.CXX);
  const headerGuardName = util.getHeaderGuardName(interfaceJson);
  const headerGuard = `__POSEMESH_${headerGuardName}_HPP__`;

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += '\n';
  code += `#ifndef ${headerGuard}\n`;
  code += `#define ${headerGuard}\n`;
  code += '\n';
  code += '#include "API.hpp"\n';
  code += '\n';
  code += 'namespace psm {\n';
  code += '\n';
  code += `class ${name} {\n`;

  let publicMethods = '', publicFuncs = '', publicMembVars = '';
  let protectedMethods = '', protectedFuncs = '', protectedMembVars = '';
  let privateMethods = '', privateFuncs = '', privateMembVars = '';

  for (const propertyJson of util.getProperties(interfaceJson)) {
    const propName = util.getPropertyName(propertyJson, util.CXX);
    const propType = util.getPropertyType(propertyJson, util.CXX);
    if (util.getPropertyHasMemberVar(propertyJson)) {
      privateMembVars += `    ${propType} ${propName};\n`;
    }
    if (propertyJson.hasGetter) {
      const getterConstExt = propertyJson.getterConst ? ' const' : '';
      const getterNoexceptExt = propertyJson.getterNoexcept ? ' noexcept' : '';
      const getterName = util.getPropertyGetterName(propertyJson, util.CXX);
      const getterType = util.getPropertyTypeForGetter(propertyJson, util.CXX);
      const getterMode = util.getPropertyGetterMode(propertyJson);
      const getterVirtualPfx = getterMode !== util.MethodMode.regular ? 'virtual ' : '';
      const getterVirtualExt = getterMode === util.MethodMode.pureVirtual ? ' = 0' : (getterMode === util.MethodMode.override ? ' override' : '');
      const getter = `    ${getterVirtualPfx}${getterType} PSM_API ${getterName}()${getterConstExt}${getterNoexceptExt}${getterVirtualExt};\n`;
      const getterVisibility = util.getPropertyGetterVisibility(propertyJson);
      switch (getterVisibility) {
        case util.Visibility.public:
          publicMethods += getter;
          break;
        case util.Visibility.protected:
          protectedMethods += getter;
          break;
        case util.Visibility.private:
          privateMethods += getter;
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
      const setter = `    ${setterVirtualPfx}void PSM_API ${setterName}(${setterType} ${setterArgName})${setterConstExt}${setterNoexceptExt}${setterVirtualExt};\n`;
      const setterVisibility = util.getPropertySetterVisibility(propertyJson);
      switch (setterVisibility) {
        case util.Visibility.public:
          publicMethods += setter;
          break;
        case util.Visibility.protected:
          protectedMethods += setter;
          break;
        case util.Visibility.private:
          privateMethods += setter;
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
  return code;
}

function generateSource(interfaceName, interfaceJson) {
  const name = util.getLangClassName(interfaceJson, util.CXX);

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += '\n';
  code += `#include <Posemesh/${interfaceName}.hpp>\n`;
  code += '\n';
  code += 'namespace psm {\n';
  code += '\n';

  let publicMethods = '', publicFuncs = '';
  let protectedMethods = '', protectedFuncs = '';
  let privateMethods = '', privateFuncs = '';

  for (const propertyJson of util.getProperties(interfaceJson)) {
    const propName = util.getPropertyName(propertyJson, util.CXX);
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
      getter += '\n';
      const getterVisibility = util.getPropertyGetterVisibility(propertyJson);
      switch (getterVisibility) {
        case util.Visibility.public:
          publicMethods += getter;
          break;
        case util.Visibility.protected:
          protectedMethods += getter;
          break;
        case util.Visibility.private:
          privateMethods += getter;
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
      setter += '\n';
      const setterVisibility = util.getPropertySetterVisibility(propertyJson);
      switch (setterVisibility) {
        case util.Visibility.public:
          publicMethods += setter;
          break;
        case util.Visibility.protected:
          protectedMethods += setter;
          break;
        case util.Visibility.private:
          privateMethods += setter;
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

  if (protectedFuncs.length > 0) {
    if (protected.length > 0) {
      protected += '\n';
    }
    protected += protectedFuncs;
  }

  if (privateFuncs.length > 0) {
    if (private.length > 0) {
      private += '\n';
    }
    private += privateFuncs;
  }

  if (public.length > 0) {
    code += public;
  }
  if (protected.length > 0) {
    if (public.length > 0) {
      code += '\n';
    }
    code += protected;
  }
  if (private.length > 0) {
    if (public.length > 0 || protected.length > 0) {
      code += '\n';
    }
    code += private;
  }

  code += '}\n';
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
