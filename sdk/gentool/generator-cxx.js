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

  let publicCtors = '', publicOperators = '', publicMethods = '', publicFuncs = '', publicMembVars = '', publicStatVars = '';
  let protectedCtors = '', protectedOperators = '', protectedMethods = '', protectedFuncs = '', protectedMembVars = '', protectedStatVars = '';
  let privateCtors = '', privateOperators = '', privateMethods = '', privateFuncs = '', privateMembVars = '', privateStatVars = '', privateFriends = '';

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
  let cCtor = undefined, cAsOp = undefined;
  switch (cCtorDefinition) {
    case util.ConstructorDefinition.defined:
    case util.ConstructorDefinition.default:
      cCtor = `    PSM_API ${name}(const ${name}& ${cCtorMainArgName})${cCtorNoexceptExt};\n`;
      cAsOp = `    ${name}& PSM_API operator=(const ${name}& ${cCtorMainArgName})${cCtorNoexceptExt};\n`;
      break;
    case util.ConstructorDefinition.deleted:
      cCtor = `    ${name}(const ${name}& ${cCtorMainArgName})${cCtorNoexceptExt} = delete;\n`;
      cAsOp = `    ${name}& operator=(const ${name}& ${cCtorMainArgName})${cCtorNoexceptExt} = delete;\n`;
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
  if (typeof cAsOp !== 'undefined') {
    switch (cCtorVisibility) {
      case util.Visibility.public:
        publicOperators += cAsOp;
        break;
      case util.Visibility.protected:
        protectedOperators += cAsOp;
        break;
      case util.Visibility.private:
        privateOperators += cAsOp;
        break;
    }
  }

  const moveConstructor = util.getClassMoveConstructor(interfaceJson);
  const mCtorDefinition = util.getConstructorDefinition(moveConstructor);
  const mCtorVisibility = util.getConstructorVisibility(moveConstructor);
  const mCtorNoexcept = util.getConstructorNoexcept(moveConstructor);
  const mCtorNoexceptExt = mCtorNoexcept ? ' noexcept' : '';
  const mCtorMainArgName = util.getCopyOrMoveConstructorMainArgName(moveConstructor, util.CXX);
  let mCtor = undefined, mAsOp = undefined;
  switch (mCtorDefinition) {
    case util.ConstructorDefinition.defined:
    case util.ConstructorDefinition.default:
      mCtor = `    PSM_API ${name}(${name}&& ${mCtorMainArgName})${mCtorNoexceptExt};\n`;
      mAsOp = `    ${name}& PSM_API operator=(${name}&& ${mCtorMainArgName})${mCtorNoexceptExt};\n`;
      break;
    case util.ConstructorDefinition.deleted:
      mCtor = `    ${name}(${name}&& ${mCtorMainArgName})${mCtorNoexceptExt} = delete;\n`;
      mAsOp = `    ${name}& operator=(${name}&& ${mCtorMainArgName})${mCtorNoexceptExt} = delete;\n`;
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
  if (typeof mAsOp !== 'undefined') {
    switch (mCtorVisibility) {
      case util.Visibility.public:
        publicOperators += mAsOp;
        break;
      case util.Visibility.protected:
        protectedOperators += mAsOp;
        break;
      case util.Visibility.private:
        privateOperators += mAsOp;
        break;
    }
  }

  const destructor = interfaceJson.destructor;
  const dtorVirtual = util.getDestructorVirtual(destructor);
  const dtorVirtualPfx = dtorVirtual ? 'virtual ' : '';
  const dtorDefinition = util.getDestructorDefinition(destructor);
  const dtorVisibility = util.getDestructorVisibility(destructor);
  let dtor = undefined;
  switch (dtorDefinition) {
    case util.DestructorDefinition.defined:
    case util.DestructorDefinition.default:
      dtor = `    ${dtorVirtualPfx}PSM_API ~${name}();\n`;
      break;
  }
  if (typeof dtor !== 'undefined') {
    switch (dtorVisibility) {
      case util.Visibility.public:
        publicCtors += dtor;
        break;
      case util.Visibility.protected:
        protectedCtors += dtor;
        break;
      case util.Visibility.private:
        privateCtors += dtor;
        break;
    }
  }

  const equalityOperator = interfaceJson.equalityOperator;
  if (equalityOperator.defined) {
    const argName = util.getStyleName('name', interfaceJson, util.camelBack);
    publicOperators += `    bool PSM_API operator==(const ${name}& ${argName}) const noexcept;\n`;
    publicOperators += `    bool PSM_API operator!=(const ${name}& ${argName}) const noexcept;\n`;
  }

  for (const propertyJson of util.getProperties(interfaceJson)) {
    const propName = util.getPropertyName(propertyJson, util.CXX);
    let shouldAddIncludes = false;
    const propType = util.getPropertyType(propertyJson, util.CXX);
    const propStatic = util.getPropertyStatic(propertyJson);
    const propStaticPfx = propStatic ? 'static ' : '';
    if (util.getPropertyHasMemberVar(propertyJson)) {
      shouldAddIncludes = true;
      const m = `    ${propStaticPfx}${propType} ${propName};\n`;
      if (propStatic) {
        privateStatVars += m;
      } else {
        privateMembVars += m;
      }
    }
    if (propertyJson.hasGetter) {
      shouldAddIncludes = true;
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
      shouldAddIncludes = true;
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
    if (shouldAddIncludes) {
      const propTypeRaw = propertyJson.type;
      if (util.isIntType(propTypeRaw)) {
        includesFirst.add('#include <cstdint>');
      }
    }
  }

  const hashOperator = interfaceJson.hashOperator;
  if (hashOperator.defined) {
    privateFriends += `    friend struct std::hash<${name}>;\n`;
    includesFirst.add('#include <functional>');
  }

  let public = publicCtors, protected = protectedCtors, private = privateCtors;

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

  if (protectedOperators.length > 0) {
    if (protected.length > 0) {
      protected += '\n';
    }
    protected += protectedOperators;
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
  if (privateFriends.length > 0) {
    if (private.length > 0) {
      private += '\n';
    }
    private += privateFriends;
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

  if (hashOperator.defined) {
    const argName = util.getStyleName('name', interfaceJson, util.camelBack);
    code += `\n`;
    code += `namespace std {\n`;
    code += `\n`;
    code += `template <>\n`;
    code += `struct hash<psm::${name}> {\n`;
    code += `    std::size_t PSM_API operator()(const psm::${name}& ${argName}) const noexcept;\n`;
    code += `};\n`;
    code += `\n`;
    code += `}\n`;
    includesFirst.add('#include <functional>');
  }

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

  let publicCtors = '', publicOperators = '', publicMethods = '', publicFuncs = '', publicStatVars = '';
  let protectedCtors = '', protectedOperators = '', protectedMethods = '', protectedFuncs = '', protectedStatVars = '';
  let privateCtors = '', privateOperators = '', privateMethods = '', privateFuncs = '', privateStatVars = '';

  const parameterlessConstructor = util.getClassParameterlessConstructor(interfaceJson);
  const pCtorCustom = util.getConstructorCustom(parameterlessConstructor);
  if (!pCtorCustom) {
    const pCtorDefinition = util.getConstructorDefinition(parameterlessConstructor);
    const pCtorVisibility = util.getConstructorVisibility(parameterlessConstructor);
    const pCtorNoexcept = util.getConstructorNoexcept(parameterlessConstructor);
    const pCtorNoexceptExt = pCtorNoexcept ? ' noexcept' : '';
    let pCtor = undefined;
    switch (pCtorDefinition) {
      case util.ConstructorDefinition.defined:
        {
          let membInitExt = '';
          let initializeInBodyCode = [];
          for (const initProp of util.getConstructorInitializedProperties(parameterlessConstructor)) {
            const initPropName = initProp.name;
            const initPropValue = initProp.value;
            if (initPropValue.length > 0) {
              let foundProperty = undefined;
              for (const prop of util.getProperties(interfaceJson)) {
                if (prop.name === initPropName) {
                  foundProperty = prop;
                  break;
                }
              }
              const propName = util.getPropertyName(foundProperty, util.CXX);
              if (initProp.initializeInBody) {
                initializeInBodyCode.push(`${propName} = ${initPropValue};`);
              } else {
                if (membInitExt.length > 0) {
                  membInitExt += `\n    , ${propName}(${initPropValue})`;
                } else {
                  membInitExt += `\n    : ${propName}(${initPropValue})`;
                }
              }
              if (new RegExp('\\bstd\\s*::\\s*move\\b').test(initPropValue)) {
                includesFirst.add('#include <utility>');
              }
            }
          }
          let bodyExt = ' { }';
          const codeFront = util.getConstructorCodeFront(parameterlessConstructor);
          const codeBack = util.getConstructorCodeBack(parameterlessConstructor);
          if (codeFront.length > 0 || initializeInBodyCode.length > 0 || codeBack.length > 0) {
            bodyExt = '\n{';
            for (const line of codeFront) {
              if (line.length > 0) {
                bodyExt += `\n    ${line}`;
              } else {
                bodyExt += '\n';
              }
            }
            for (const line of initializeInBodyCode) {
              if (line.length > 0) {
                bodyExt += `\n    ${line}`;
              } else {
                bodyExt += '\n';
              }
            }
            for (const line of codeBack) {
              if (line.length > 0) {
                bodyExt += `\n    ${line}`;
              } else {
                bodyExt += '\n';
              }
            }
            bodyExt += '\n}';
          } else if (membInitExt.length > 0) {
            bodyExt = '\n{\n}';
          }
          pCtor = `${name}::${name}()${pCtorNoexceptExt}${membInitExt}${bodyExt}\n`;
        }
        break;
      case util.ConstructorDefinition.default:
        pCtor = `${name}::${name}()${pCtorNoexceptExt} = default;\n`;
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
  }

  const copyConstructor = util.getClassCopyConstructor(interfaceJson);
  const cCtorCustom = util.getConstructorCustom(copyConstructor);
  const cCtorCustomOperator = util.getConstructorCustomOperator(copyConstructor);
  if (!cCtorCustom || !cCtorCustomOperator) {
    const cCtorDefinition = util.getConstructorDefinition(copyConstructor);
    const cCtorVisibility = util.getConstructorVisibility(copyConstructor);
    const cCtorNoexcept = util.getConstructorNoexcept(copyConstructor);
    const cCtorNoexceptExt = cCtorNoexcept ? ' noexcept' : '';
    const cCtorMainArgName = util.getCopyOrMoveConstructorMainArgName(copyConstructor, util.CXX);
    let cCtor = undefined, cAsOp = undefined;
    switch (cCtorDefinition) {
      case util.ConstructorDefinition.defined:
        {
          let membInitExt = '';
          let initializeInBodyCode = [];
          let initializeInBodyCodeAsOp = [];
          for (const initProp of util.getConstructorInitializedProperties(copyConstructor)) {
            const initPropName = initProp.name;
            const initPropValue = initProp.value;
            if (initPropValue.length > 0) {
              let foundProperty = undefined;
              for (const prop of util.getProperties(interfaceJson)) {
                if (prop.name === initPropName) {
                  foundProperty = prop;
                  break;
                }
              }
              const propName = util.getPropertyName(foundProperty, util.CXX);
              const initPropValueEval = initPropValue.replaceAll(initProp.valuePlaceholder, `${cCtorMainArgName}.${propName}`);
              if (initProp.initializeInBody) {
                initializeInBodyCode.push(`${propName} = ${initPropValueEval};`);
              } else {
                if (membInitExt.length > 0) {
                  membInitExt += `\n    , ${propName}(${initPropValueEval})`;
                } else {
                  membInitExt += `\n    : ${propName}(${initPropValueEval})`;
                }
              }
              initializeInBodyCodeAsOp.push(`${propName} = ${initPropValueEval};`);
              if (new RegExp('\\bstd\\s*::\\s*move\\b').test(initPropValueEval)) {
                includesFirst.add('#include <utility>');
              }
            } else {
              initializeInBodyCodeAsOp.push(`${propName} = {};`);
            }
          }
          let bodyExt = ' { }';
          const codeFront = util.getConstructorCodeFront(copyConstructor);
          const codeBack = util.getConstructorCodeBack(copyConstructor);
          if (codeFront.length > 0 || initializeInBodyCode.length > 0 || codeBack.length > 0) {
            bodyExt = '\n{';
            for (const line of codeFront) {
              if (line.length > 0) {
                bodyExt += `\n    ${line}`;
              } else {
                bodyExt += '\n';
              }
            }
            for (const line of initializeInBodyCode) {
              if (line.length > 0) {
                bodyExt += `\n    ${line}`;
              } else {
                bodyExt += '\n';
              }
            }
            for (const line of codeBack) {
              if (line.length > 0) {
                bodyExt += `\n    ${line}`;
              } else {
                bodyExt += '\n';
              }
            }
            bodyExt += '\n}';
          } else if (membInitExt.length > 0) {
            bodyExt = '\n{\n}';
          }
          let bodyAsOpExt = '\n{';
          bodyAsOpExt += `\n    if (&${cCtorMainArgName} == this)`;
          bodyAsOpExt += `\n        return *this;`;
          for (const line of util.getConstructorOperatorCodeFront(copyConstructor)) {
            if (line.length > 0) {
              bodyAsOpExt += `\n    ${line}`;
            } else {
              bodyAsOpExt += '\n';
            }
          }
          for (const line of initializeInBodyCodeAsOp) {
            if (line.length > 0) {
              bodyAsOpExt += `\n    ${line}`;
            } else {
              bodyAsOpExt += '\n';
            }
          }
          for (const line of util.getConstructorOperatorCodeBack(copyConstructor)) {
            if (line.length > 0) {
              bodyAsOpExt += `\n    ${line}`;
            } else {
              bodyAsOpExt += '\n';
            }
          }
          bodyAsOpExt += `\n    return *this;`;
          bodyAsOpExt += '\n}';
          if (!cCtorCustom) {
            cCtor = `${name}::${name}(const ${name}& ${cCtorMainArgName})${cCtorNoexceptExt}${membInitExt}${bodyExt}\n`;
          }
          if (!cCtorCustomOperator) {
            cAsOp = `${name}& ${name}::operator=(const ${name}& ${cCtorMainArgName})${cCtorNoexceptExt}${bodyAsOpExt}\n`;
          }
        }
        break;
      case util.ConstructorDefinition.default:
        if (!cCtorCustom) {
          cCtor = `${name}::${name}(const ${name}& ${cCtorMainArgName})${cCtorNoexceptExt} = default;\n`;
        }
        if (!cCtorCustomOperator) {
          cAsOp = `${name}& ${name}::operator=(const ${name}& ${cCtorMainArgName})${cCtorNoexceptExt} = default;\n`;
        }
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
    if (typeof cAsOp !== 'undefined') {
      switch (cCtorVisibility) {
        case util.Visibility.public:
          if (publicOperators.length > 0) { publicOperators += '\n'; }
          publicOperators += cAsOp;
          break;
        case util.Visibility.protected:
          if (protectedOperators.length > 0) { protectedOperators += '\n'; }
          protectedOperators += cAsOp;
          break;
        case util.Visibility.private:
          if (privateOperators.length > 0) { privateOperators += '\n'; }
          privateOperators += cAsOp;
          break;
      }
    }
  }

  const moveConstructor = util.getClassMoveConstructor(interfaceJson);
  const mCtorCustom = util.getConstructorCustom(moveConstructor);
  const mCtorCustomOperator = util.getConstructorCustomOperator(moveConstructor);
  if (!mCtorCustom || !mCtorCustomOperator) {
    const mCtorDefinition = util.getConstructorDefinition(moveConstructor);
    const mCtorVisibility = util.getConstructorVisibility(moveConstructor);
    const mCtorNoexcept = util.getConstructorNoexcept(moveConstructor);
    const mCtorNoexceptExt = mCtorNoexcept ? ' noexcept' : '';
    const mCtorMainArgName = util.getCopyOrMoveConstructorMainArgName(moveConstructor, util.CXX);
    let mCtor = undefined, mAsOp = undefined;
    switch (mCtorDefinition) {
      case util.ConstructorDefinition.defined:
        {
          let membInitExt = '';
          let initializeInBodyCode = [];
          let initializeInBodyCodeAsOp = [];
          for (const initProp of util.getConstructorInitializedProperties(moveConstructor)) {
            const initPropName = initProp.name;
            const initPropValue = initProp.value;
            if (initPropValue.length > 0) {
              let foundProperty = undefined;
              for (const prop of util.getProperties(interfaceJson)) {
                if (prop.name === initPropName) {
                  foundProperty = prop;
                  break;
                }
              }
              const propName = util.getPropertyName(foundProperty, util.CXX);
              const initPropValueEval = initPropValue.replaceAll(initProp.valuePlaceholder, `${mCtorMainArgName}.${propName}`);
              if (initProp.initializeInBody) {
                initializeInBodyCode.push(`${propName} = ${initPropValueEval};`);
              } else {
                if (membInitExt.length > 0) {
                  membInitExt += `\n    , ${propName}(${initPropValueEval})`;
                } else {
                  membInitExt += `\n    : ${propName}(${initPropValueEval})`;
                }
              }
              initializeInBodyCodeAsOp.push(`${propName} = ${initPropValueEval};`);
              if (new RegExp('\\bstd\\s*::\\s*move\\b').test(initPropValueEval)) {
                includesFirst.add('#include <utility>');
              }
            } else {
              initializeInBodyCodeAsOp.push(`${propName} = {};`);
            }
          }
          let bodyExt = ' { }';
          const codeFront = util.getConstructorCodeFront(moveConstructor);
          const codeBack = util.getConstructorCodeBack(moveConstructor);
          if (codeFront.length > 0 || initializeInBodyCode.length > 0 || codeBack.length > 0) {
            bodyExt = '\n{';
            for (const line of codeFront) {
              if (line.length > 0) {
                bodyExt += `\n    ${line}`;
              } else {
                bodyExt += '\n';
              }
            }
            for (const line of initializeInBodyCode) {
              if (line.length > 0) {
                bodyExt += `\n    ${line}`;
              } else {
                bodyExt += '\n';
              }
            }
            for (const line of codeBack) {
              if (line.length > 0) {
                bodyExt += `\n    ${line}`;
              } else {
                bodyExt += '\n';
              }
            }
            bodyExt += '\n}';
          } else if (membInitExt.length > 0) {
            bodyExt = '\n{\n}';
          }
          let bodyAsOpExt = '\n{';
          bodyAsOpExt += `\n    if (&${mCtorMainArgName} == this)`;
          bodyAsOpExt += `\n        return *this;`;
          for (const line of util.getConstructorOperatorCodeFront(moveConstructor)) {
            if (line.length > 0) {
              bodyAsOpExt += `\n    ${line}`;
            } else {
              bodyAsOpExt += '\n';
            }
          }
          for (const line of initializeInBodyCodeAsOp) {
            if (line.length > 0) {
              bodyAsOpExt += `\n    ${line}`;
            } else {
              bodyAsOpExt += '\n';
            }
          }
          for (const line of util.getConstructorOperatorCodeBack(moveConstructor)) {
            if (line.length > 0) {
              bodyAsOpExt += `\n    ${line}`;
            } else {
              bodyAsOpExt += '\n';
            }
          }
          bodyAsOpExt += `\n    return *this;`;
          bodyAsOpExt += '\n}';
          if (!mCtorCustom) {
            mCtor = `${name}::${name}(${name}&& ${mCtorMainArgName})${mCtorNoexceptExt}${membInitExt}${bodyExt}\n`;
          }
          if (!mCtorCustomOperator) {
            mAsOp = `${name}& ${name}::operator=(${name}&& ${mCtorMainArgName})${mCtorNoexceptExt}${bodyAsOpExt}\n`;
          }
        }
        break;
      case util.ConstructorDefinition.default:
        if (!mCtorCustom) {
          mCtor = `${name}::${name}(${name}&& ${mCtorMainArgName})${mCtorNoexceptExt} = default;\n`;
        }
        if (!mCtorCustomOperator) {
          mAsOp = `${name}& ${name}::operator=(${name}&& ${mCtorMainArgName})${mCtorNoexceptExt} = default;\n`;
        }
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
    if (typeof mAsOp !== 'undefined') {
      switch (mCtorVisibility) {
        case util.Visibility.public:
          if (publicOperators.length > 0) { publicOperators += '\n'; }
          publicOperators += mAsOp;
          break;
        case util.Visibility.protected:
          if (protectedOperators.length > 0) { protectedOperators += '\n'; }
          protectedOperators += mAsOp;
          break;
        case util.Visibility.private:
          if (privateOperators.length > 0) { privateOperators += '\n'; }
          privateOperators += mAsOp;
          break;
      }
    }
  }

  const destructor = interfaceJson.destructor;
  const dtorCode = util.getDestructorCode(destructor);
  const dtorDefinition = util.getDestructorDefinition(destructor);
  const dtorVisibility = util.getDestructorVisibility(destructor);
  const dtorCustom = util.getDestructorCustom(destructor);
  let dtor = undefined;
  if (!dtorCustom) {
    switch (dtorDefinition) {
      case util.DestructorDefinition.defined:
        dtor = `${name}::~${name}()\n`;
        dtor += '{\n';
        for (const line of dtorCode) {
          if (line.length > 0) {
            dtor += `    ${line}\n`;
          } else {
            dtor += '\n';
          }
        }
        dtor += '}\n';
        break;
      case util.DestructorDefinition.default:
        dtor = `${name}::~${name}() = default;\n`;
        break;
    }
  }
  if (typeof dtor !== 'undefined') {
    switch (dtorVisibility) {
      case util.Visibility.public:
        if (publicCtors.length > 0) { publicCtors += '\n'; }
        publicCtors += dtor;
        break;
      case util.Visibility.protected:
        if (protectedCtors.length > 0) { protectedCtors += '\n'; }
        protectedCtors += dtor;
        break;
      case util.Visibility.private:
        if (privateCtors.length > 0) { privateCtors += '\n'; }
        privateCtors += dtor;
        break;
    }
  }

  const equalityOperator = interfaceJson.equalityOperator;
  if (equalityOperator.defined) {
    const argName = util.getStyleName('name', interfaceJson, util.camelBack);
    let eqOp = undefined, neOp = undefined;

    if (!equalityOperator.custom) {
      eqOp = `bool ${name}::operator==(const ${name}& ${argName}) const noexcept\n`;
      eqOp += `{\n`;
      if (equalityOperator.comparePointers) {
        eqOp += `    return this == &${argName};\n`;
      } else {
        for (const comparedPropertyJson of equalityOperator.comparedProperties) {
          const name = comparedPropertyJson.name;
          const useGetter = comparedPropertyJson.useGetter;
          const comparator = comparedPropertyJson.comparator;
          const comparatorClassInstancePlaceholder = comparedPropertyJson.comparatorClassInstancePlaceholder;
          const comparatorPropertyPlaceholder = comparedPropertyJson.comparatorPropertyPlaceholder;

          let foundPropertyJson = undefined;
          for (const propertyJson of util.getProperties(interfaceJson)) {
            if (propertyJson.name === name) {
              foundPropertyJson = propertyJson;
              break;
            }
          }

          let line = comparator.replaceAll(comparatorClassInstancePlaceholder, argName);
          if (useGetter) {
            const getterName = util.getPropertyGetterName(foundPropertyJson, util.CXX);
            line = line.replaceAll(comparatorPropertyPlaceholder, `${getterName}()`);
          } else {
            const propName = util.getPropertyName(foundPropertyJson, util.CXX);
            line = line.replaceAll(comparatorPropertyPlaceholder, `${propName}`);
          }

          eqOp += `    if (!(${line})) {\n`;
          eqOp += `        return false;\n`;
          eqOp += `    }\n`;
        }
        eqOp += `    return true;\n`;
      }
      eqOp += `}\n`;
    }

    if (!equalityOperator.customInequality) {
      neOp = `bool ${name}::operator!=(const ${name}& ${argName}) const noexcept\n`;
      neOp += `{\n`;
      neOp += `    return !(*this == ${argName});\n`;
      neOp += `}\n`;
    }

    if (typeof eqOp !== 'undefined') {
      if (publicOperators.length > 0) { publicOperators += '\n'; }
      publicOperators += eqOp;
    }
    if (typeof neOp !== 'undefined') {
      if (publicOperators.length > 0) { publicOperators += '\n'; }
      publicOperators += neOp;
    }
  }

  for (const propertyJson of util.getProperties(interfaceJson)) {
    const propName = util.getPropertyName(propertyJson, util.CXX);
    const propType = util.getPropertyType(propertyJson, util.CXX);
    const propStatic = util.getPropertyStatic(propertyJson);
    const propDefaultValue = util.getPropertyDefaultValue(propertyJson);
    if (propStatic) {
      const propDefaultValueSpaced = propDefaultValue.length > 0 ? ` ${propDefaultValue} ` : '';
      privateStatVars += `${propType} ${name}::${propName} {${propDefaultValueSpaced}};\n`;
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
  if (publicStatVars.length > 0) {
    if (public.length > 0) {
      public += '\n';
    }
    public += publicStatVars;
  }

  if (protectedOperators.length > 0) {
    if (protected.length > 0) {
      protected += '\n';
    }
    protected += protectedOperators;
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

  const hashOperator = interfaceJson.hashOperator;
  if (hashOperator.defined && !hashOperator.custom) {
    const argName = util.getStyleName('name', interfaceJson, util.camelBack);
    code += `\n`;
    code += `namespace std {\n`;
    code += `\n`;
    code += `std::size_t hash<psm::${name}>::operator()(const psm::${name}& ${argName}) const noexcept\n`;
    code += `{\n`;
    if (hashOperator.usePointerAsHash) {
      code += `    return hash<const psm::${name}*> {}(&${argName});\n`;
    } else {
      code += `    std::size_t result = 0;\n`;
      for (const hashedPropertyJson of hashOperator.hashedProperties) {
        const name = hashedPropertyJson.name;
        const useGetter = hashedPropertyJson.useGetter;
        const hasher = hashedPropertyJson.hasher;
        const hasherPlaceholder = hashedPropertyJson.hasherPlaceholder;

        let foundPropertyJson = undefined;
        for (const propertyJson of util.getProperties(interfaceJson)) {
          if (propertyJson.name === name) {
            foundPropertyJson = propertyJson;
            break;
          }
        }

        let line = hasher;
        if (useGetter) {
          const getterName = util.getPropertyGetterName(foundPropertyJson, util.CXX);
          line = line.replaceAll(hasherPlaceholder, `${argName}.${getterName}()`);
        } else {
          const propName = util.getPropertyName(foundPropertyJson, util.CXX);
          line = line.replaceAll(hasherPlaceholder, `${argName}.${propName}`);
        }

        code += `    result ^= (${line}) + 0x9e3779b9 + (result << 6) + (result >> 2);\n`;
      }
      code += `    return result;\n`;
    }
    code += `}\n`;
    code += `\n`;
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

function generateInterfaceCXX(interfaceName, interfaceJson) {
  const headerFilePath = path.resolve(__dirname, '..', 'include', 'Posemesh', `${interfaceName}.hpp`);
  const sourceFilePath = path.resolve(__dirname, '..', 'src', `${interfaceName}.gen.cpp`);

  let headerCode = generateHeader(interfaceName, interfaceJson);
  let sourceCode = generateSource(interfaceName, interfaceJson);

  fs.writeFileSync(headerFilePath, headerCode, 'utf8');
  fs.writeFileSync(sourceFilePath, sourceCode, 'utf8');
}

module.exports = generateInterfaceCXX;
