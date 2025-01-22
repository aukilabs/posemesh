const fs = require('fs');
const path = require('path');
const util = require('./util');

function generateHeader(interfaceName, interfaceJson) {
  const name = util.getLangClassName(interfaceJson, util.CXX);
  const classStatic = util.getClassStatic(interfaceJson);
  const classFinal = util.getClassFinal(interfaceJson);
  const classFinalExt = classFinal ? ' final' : '';
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
  code += `class ${name}${classFinalExt} {\n`;

  let publicCtors = '', publicMethods = '', publicFuncs = '', publicMembVars = '', publicStatVars = '';
  let protectedCtors = '', protectedMethods = '', protectedFuncs = '', protectedMembVars = '', protectedStatVars = '';
  let privateCtors = '', privateMethods = '', privateFuncs = '', privateMembVars = '', privateStatVars = '';

  const parameterlessConstructor = util.getClassParameterlessConstructor(interfaceJson);
  const pCtorDefinition = util.getConstructorDefinition(parameterlessConstructor);
  const pCtorVisibility = util.getConstructorVisibility(parameterlessConstructor);
  const pCtorNoexcept = util.getConstructorNoexcept(parameterlessConstructor);
  const pCtorNoexceptExt = pCtorNoexcept ? ' noexcept' : '';
  let pCtor = undefined;
  switch (pCtorDefinition) {
    case util.ConstructorDefinition.defined:
    case util.ConstructorDefinition.default:
      pCtor = `    PSM_API ${name}()${pCtorNoexceptExt};\n`;
      break;
    case util.ConstructorDefinition.deleted:
      pCtor = `    ${name}()${pCtorNoexceptExt} = delete;\n`;
      break;
  }
  if (typeof pCtor !== 'undefined') {
    switch (pCtorVisibility) {
      case util.Visibility.public:
        publicCtors += pCtor;
        break;
      case util.Visibility.protected:
        protectedCtors += pCtor;
        break;
      case util.Visibility.private:
        privateCtors += pCtor;
        break;
    }
  }

  const copyConstructor = util.getClassCopyConstructor(interfaceJson);
  const cCtorDefinition = util.getConstructorDefinition(copyConstructor);
  const cCtorVisibility = util.getConstructorVisibility(copyConstructor);
  const cCtorNoexcept = util.getConstructorNoexcept(copyConstructor);
  const cCtorNoexceptExt = cCtorNoexcept ? ' noexcept' : '';
  const cCtorMainArgName = util.getCopyOrMoveConstructorMainArgName(copyConstructor, util.CXX);
  let cCtor = undefined;
  switch (cCtorDefinition) {
    case util.ConstructorDefinition.defined:
    case util.ConstructorDefinition.default:
      cCtor = `    PSM_API ${name}(const ${name}& ${cCtorMainArgName})${cCtorNoexceptExt};\n`;
      break;
    case util.ConstructorDefinition.deleted:
      cCtor = `    ${name}(const ${name}& ${cCtorMainArgName})${cCtorNoexceptExt} = delete;\n`;
      break;
  }
  if (typeof cCtor !== 'undefined') {
    switch (cCtorVisibility) {
      case util.Visibility.public:
        publicCtors += cCtor;
        break;
      case util.Visibility.protected:
        protectedCtors += cCtor;
        break;
      case util.Visibility.private:
        privateCtors += cCtor;
        break;
    }
  }

  const moveConstructor = util.getClassMoveConstructor(interfaceJson);
  const mCtorDefinition = util.getConstructorDefinition(moveConstructor);
  const mCtorVisibility = util.getConstructorVisibility(moveConstructor);
  const mCtorNoexcept = util.getConstructorNoexcept(moveConstructor);
  const mCtorNoexceptExt = mCtorNoexcept ? ' noexcept' : '';
  const mCtorMainArgName = util.getCopyOrMoveConstructorMainArgName(moveConstructor, util.CXX);
  let mCtor = undefined;
  switch (mCtorDefinition) {
    case util.ConstructorDefinition.defined:
    case util.ConstructorDefinition.default:
      mCtor = `    PSM_API ${name}(${name}&& ${mCtorMainArgName})${mCtorNoexceptExt};\n`;
      break;
    case util.ConstructorDefinition.deleted:
      mCtor = `    ${name}(${name}&& ${mCtorMainArgName})${mCtorNoexceptExt} = delete;\n`;
      break;
  }
  if (typeof mCtor !== 'undefined') {
    switch (mCtorVisibility) {
      case util.Visibility.public:
        publicCtors += mCtor;
        break;
      case util.Visibility.protected:
        protectedCtors += mCtor;
        break;
      case util.Visibility.private:
        privateCtors += mCtor;
        break;
    }
  }

  for (const propertyJson of util.getProperties(interfaceJson)) {
    const propName = util.getPropertyName(propertyJson, util.CXX);
    const propTypeRaw = propertyJson.type;
    if (util.isIntType(propTypeRaw)) {
      includesFirst.add('#include <cstdint>');
    }
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

  let public = publicCtors, protected = protectedCtors, private = privateCtors;

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

  if (protectedMethods.length > 0) {
    if (protected.length > 0) {
      protected += '\n';
    }
    protected += protectedMethods;
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
  const classStatic = util.getClassStatic(interfaceJson);

  let includesFirst = new Set([`#include <Posemesh/${interfaceName}.hpp>`]), includesSecond = new Set([]);

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += '%INCLUDES%\n';
  code += 'namespace psm {\n';
  code += '\n';

  let publicCtors = '', publicMethods = '', publicFuncs = '', publicStatVars = '';
  let protectedCtors = '', protectedMethods = '', protectedFuncs = '', protectedStatVars = '';
  let privateCtors = '', privateMethods = '', privateFuncs = '', privateStatVars = '';

  const parameterlessConstructor = util.getClassParameterlessConstructor(interfaceJson);
  const pCtorDefinition = util.getConstructorDefinition(parameterlessConstructor);
  const pCtorVisibility = util.getConstructorVisibility(parameterlessConstructor);
  const pCtorNoexcept = util.getConstructorNoexcept(parameterlessConstructor);
  const pCtorNoexceptExt = pCtorNoexcept ? ' noexcept' : '';
  let pCtor = undefined;
  switch (pCtorDefinition) {
    case util.ConstructorDefinition.defined:
      pCtor = `${name}::${name}()${pCtorNoexceptExt} {}\n`;
      break;
    case util.ConstructorDefinition.default:
      pCtor = `${name}::${name}()${pCtorNoexceptExt} = default;\n`;
      break;
    case util.ConstructorDefinition.deleted:
      pCtor = `${name}::${name}()${pCtorNoexceptExt} = delete;\n`;
      break;
  }
  if (typeof pCtor !== 'undefined') {
    switch (pCtorVisibility) {
      case util.Visibility.public:
        if (publicCtors.length > 0) { publicCtors += '\n'; }
        publicCtors += pCtor;
        break;
      case util.Visibility.protected:
        if (protectedCtors.length > 0) { protectedCtors += '\n'; }
        protectedCtors += pCtor;
        break;
      case util.Visibility.private:
        if (privateCtors.length > 0) { privateCtors += '\n'; }
        privateCtors += pCtor;
        break;
    }
  }

  const copyConstructor = util.getClassCopyConstructor(interfaceJson);
  const cCtorDefinition = util.getConstructorDefinition(copyConstructor);
  const cCtorVisibility = util.getConstructorVisibility(copyConstructor);
  const cCtorNoexcept = util.getConstructorNoexcept(copyConstructor);
  const cCtorNoexceptExt = cCtorNoexcept ? ' noexcept' : '';
  const cCtorMainArgName = util.getCopyOrMoveConstructorMainArgName(copyConstructor, util.CXX);
  let cCtor = undefined;
  switch (cCtorDefinition) {
    case util.ConstructorDefinition.defined:
      cCtor = `${name}::${name}(const ${name}& ${cCtorMainArgName})${cCtorNoexceptExt} {}\n`;
      break;
    case util.ConstructorDefinition.default:
      cCtor = `${name}::${name}(const ${name}& ${cCtorMainArgName})${cCtorNoexceptExt} = default;\n`;
      break;
    case util.ConstructorDefinition.deleted:
      cCtor = `${name}::${name}(const ${name}& ${cCtorMainArgName})${cCtorNoexceptExt} = delete;\n`;
      break;
  }
  if (typeof cCtor !== 'undefined') {
    switch (cCtorVisibility) {
      case util.Visibility.public:
        if (publicCtors.length > 0) { publicCtors += '\n'; }
        publicCtors += cCtor;
        break;
      case util.Visibility.protected:
        if (protectedCtors.length > 0) { protectedCtors += '\n'; }
        protectedCtors += cCtor;
        break;
      case util.Visibility.private:
        if (privateCtors.length > 0) { privateCtors += '\n'; }
        privateCtors += cCtor;
        break;
    }
  }

  const moveConstructor = util.getClassMoveConstructor(interfaceJson);
  const mCtorDefinition = util.getConstructorDefinition(moveConstructor);
  const mCtorVisibility = util.getConstructorVisibility(moveConstructor);
  const mCtorNoexcept = util.getConstructorNoexcept(moveConstructor);
  const mCtorNoexceptExt = mCtorNoexcept ? ' noexcept' : '';
  const mCtorMainArgName = util.getCopyOrMoveConstructorMainArgName(moveConstructor, util.CXX);
  let mCtor = undefined;
  switch (mCtorDefinition) {
    case util.ConstructorDefinition.defined:
      mCtor = `${name}::${name}(${name}&& ${mCtorMainArgName})${mCtorNoexceptExt} {}\n`;
      break;
    case util.ConstructorDefinition.default:
      mCtor = `${name}::${name}(${name}&& ${mCtorMainArgName})${mCtorNoexceptExt} = default;\n`;
      break;
    case util.ConstructorDefinition.deleted:
      mCtor = `${name}::${name}(${name}&& ${mCtorMainArgName})${mCtorNoexceptExt} = delete;\n`;
      break;
  }
  if (typeof mCtor !== 'undefined') {
    switch (mCtorVisibility) {
      case util.Visibility.public:
        if (publicCtors.length > 0) { publicCtors += '\n'; }
        publicCtors += mCtor;
        break;
      case util.Visibility.protected:
        if (protectedCtors.length > 0) { protectedCtors += '\n'; }
        protectedCtors += mCtor;
        break;
      case util.Visibility.private:
        if (privateCtors.length > 0) { privateCtors += '\n'; }
        privateCtors += mCtor;
        break;
    }
  }

  for (const propertyJson of util.getProperties(interfaceJson)) {
    const propName = util.getPropertyName(propertyJson, util.CXX);
    const propType = util.getPropertyType(propertyJson, util.CXX);
    const propStatic = util.getPropertyStatic(propertyJson);
    const propDefaultValue = util.getPropertyDefaultValue(propertyJson);
    if (propStatic) {
      privateStatVars += `${propType} ${name}::${propName} {${propDefaultValue}};\n`;
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

  let public = publicCtors, protected = protectedCtors, private = privateCtors;

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
  if (publicStatVars.length > 0) {
    if (public.length > 0) {
      public += '\n';
    }
    public += publicStatVars;
  }

  if (protectedMethods.length > 0) {
    if (protected.length > 0) {
      protected += '\n';
    }
    protected += protectedMethods;
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
