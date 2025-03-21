const path = require('path');
const util = require('./util');

function generateHeader(interfaces, interfaceName, interfaceJson) {
  const name = util.getLangClassName(interfaceJson, util.ObjC);
  const nameSwift = util.getLangClassName(interfaceJson, util.Swift);
  const nameCamelBack = util.getStyleName('name', interfaceJson, util.camelBack);
  const copyable = util.getClassCopyable(interfaceJson);
  const copyableExt = copyable ? '<NSCopying>' : '';
  const static = util.getClassStatic(interfaceJson);
  const managedGetterName = `managed${interfaceName}`;
  const nativeGetterName = `native${interfaceName}`;

  let importsFirst = new Set(['#import <Foundation/Foundation.h>']), importsSecond = new Set(['#import "API.h"']);
  let includesFirst = new Set([]), includesSecond = new Set([]);

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += '%INCLUDES%\n';
  code += `NS_SWIFT_NAME(${nameSwift}) PSM_API @interface ${name} : NSObject${copyableExt}\n`;

  let publicCtors = '', publicOperators = '', publicMethods = '', publicFuncs = '';

  const parameterlessConstructor = util.getClassParameterlessConstructor(interfaceJson);
  const pCtorDefinition = util.getConstructorDefinition(parameterlessConstructor);
  const pCtorVisibility = util.getConstructorVisibility(parameterlessConstructor);
  if (static || pCtorDefinition === util.ConstructorDefinition.deleted || pCtorVisibility !== util.Visibility.public) {
    publicCtors += '- (instancetype)init NS_UNAVAILABLE;\n';
  } else {
    publicCtors += '- (instancetype)init;\n';
  }

  const copyConstructor = util.getClassCopyConstructor(interfaceJson);
  const cCtorDefinition = util.getConstructorDefinition(copyConstructor);
  const cCtorVisibility = util.getConstructorVisibility(copyConstructor);
  if (!static && copyable && cCtorDefinition !== util.ConstructorDefinition.deleted && cCtorVisibility === util.Visibility.public) {
    publicCtors += `- (instancetype)initWith${interfaceName}:(${name}*)${nameCamelBack};\n`;
    publicCtors += `- (instancetype)copyWithZone:(NSZone*)zone;\n`;
  } else {
    publicCtors += `- (instancetype)copy NS_UNAVAILABLE;\n`;
  }

  if (!static) {
    publicCtors += `- (void)dealloc;\n`;
  }

  const equalityOperator = interfaceJson.equalityOperator;
  if (equalityOperator.defined) {
    publicOperators += `- (BOOL)isEqual:(id)object;\n`;
  }

  const hashOperator = interfaceJson.hashOperator;
  if (hashOperator.defined) {
    publicOperators += `- (NSUInteger)hash;\n`;
  }

  for (const propertyJson of util.getProperties(interfaceJson)) {
    let shouldAddIncludes = false;
    const propStatic = util.getPropertyStatic(propertyJson);
    if (propertyJson.hasGetter) {
      const getterName = util.getPropertyGetterName(propertyJson, util.ObjC);
      const getterType = util.getPropertyTypeForGetter(interfaces, propertyJson, util.ObjC);
      const getter = `${propStatic ? '+' : '-'} (${getterType})${getterName} NS_REFINED_FOR_SWIFT;\n`;
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
      const setterName = util.getPropertySetterName(propertyJson, util.ObjC);
      const setterType = util.getPropertyTypeForSetter(interfaces, propertyJson, util.ObjC);
      const setterArgName = util.getPropertySetterArgName(propertyJson, util.ObjC);
      const setter = `${propStatic ? '+' : '-'} (void)${setterName}:(${setterType})${setterArgName} NS_REFINED_FOR_SWIFT;\n`;
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
      if (util.isIntType(propTypeRaw)) {
        includesFirst.add('#include <stdint.h>');
      } else if (util.isClassOfAnyType(propTypeRaw)) {
        importsSecond.add(`#import "${propTypeRaw.split(':').slice(1).join(':')}.h"`);
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
    code += '\n';
    code += '#if defined(POSEMESH_BUILD)\n';
    code += `- (instancetype)initWithManaged${interfaceName}:(void*)${nameCamelBack};\n`;
    code += `- (instancetype)initWithNative${interfaceName}:(void*)${nameCamelBack};\n`;
    code += `- (void*)${managedGetterName};\n`;
    code += `- (void*)${nativeGetterName};\n`;
    code += '#endif\n';
  }
  code += '\n';
  code += '@end\n';
  const aliases = util.getLangAliases(interfaceJson, util.ObjC);
  const aliasesSwift = util.getLangAliases(interfaceJson, util.Swift);
  const aliasesPaired = aliases.map((item, index) => [item, aliasesSwift[index]]);
  if (aliasesPaired.length > 0) {
    code += '\n';
    code += '#if defined(__swift__)\n';
    for (const aliasPaired of aliasesPaired) {
      code += `typedef ${name}* __${aliasPaired[0]} NS_SWIFT_NAME(${aliasPaired[1]});\n`;
    }
    code += '#else\n';
    for (const aliasPaired of aliasesPaired) {
      code += `@compatibility_alias ${aliasPaired[0]} ${name};\n`;
    }
    code += '#endif\n';
  }

  importsFirst = Array.from(importsFirst).sort();
  importsSecond = Array.from(importsSecond).sort();
  includesFirst = Array.from(includesFirst).sort();
  includesSecond = Array.from(includesSecond).sort();
  let includes = '';
  if (importsFirst.length > 0) {
    includes += '\n';
    for (const include of importsFirst) {
      includes += include + '\n';
    }
  }
  if (includesFirst.length > 0) {
    if (importsFirst.length > 0) {
      includes += '\n';
    }
    for (const include of includesFirst) {
      includes += include + '\n';
    }
  }
  if (importsSecond.length > 0) {
    includes += '\n';
    for (const include of importsSecond) {
      includes += include + '\n';
    }
  }
  if (includesSecond.length > 0) {
    if (importsSecond.length > 0) {
      includes += '\n';
    }
    for (const include of includesSecond) {
      includes += include + '\n';
    }
  }
  code = code.replaceAll('%INCLUDES%', includes);

  return code;
}

function generateSource(interfaces, interfaceName, interfaceJson) {
  const name = util.getLangClassName(interfaceJson, util.ObjC);
  const nameCxx = util.getLangClassName(interfaceJson, util.CXX);
  const nameCamelBack = util.getStyleName('name', interfaceJson, util.camelBack);
  const nameManagedMember = `m_${nameCamelBack}`;
  const copyable = util.getClassCopyable(interfaceJson);
  const static = util.getClassStatic(interfaceJson);
  const managedGetterName = `managed${interfaceName}`;
  const nativeGetterName = `native${interfaceName}`;
  const initWithManagedName = `initWithManaged${interfaceName}`;
  const initWithNativeName = `initWithNative${interfaceName}`;

  let importsFirst = new Set([`#import <Posemesh/${interfaceName}.h>`]), importsSecond = new Set([]);
  let includesFirst = new Set([`#include <Posemesh/${interfaceName}.hpp>`]), includesSecond = new Set([]);

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += '%INCLUDES%\n';
  if (static) {
    code += `@implementation ${name}\n`;
  } else {
    code += `@implementation ${name} {\n`;
    code += `    std::shared_ptr<psm::${nameCxx}> ${nameManagedMember};\n`;
    code += '}\n';
    includesFirst.add('#include <memory>');
  }

  let publicCtors = '', publicOperators = '', publicMethods = '', publicFuncs = '';
  let privateCtors = '', privateOperators = '', privateMethods = '', privateFuncs = '';

  const parameterlessConstructor = util.getClassParameterlessConstructor(interfaceJson);
  const pCtorDefinition = util.getConstructorDefinition(parameterlessConstructor);
  const pCtorVisibility = util.getConstructorVisibility(parameterlessConstructor);
  if (!static) {
    if (pCtorDefinition !== util.ConstructorDefinition.deleted) {
      let pCtor = '- (instancetype)init\n';
      pCtor += '{\n';
      pCtor += `    auto* ${nameCamelBack} = new (std::nothrow) psm::${nameCxx};\n`;
      pCtor += `    if (!${nameCamelBack}) {\n`;
      pCtor += `        return nil;\n`;
      pCtor += `    }\n`;
      pCtor += `    self = [self ${initWithNativeName}:${nameCamelBack}];\n`;
      pCtor += `    if (!self) {\n`;
      pCtor += `        delete ${nameCamelBack};\n`;
      pCtor += `        return nil;\n`;
      pCtor += `    }\n`;
      pCtor += `    return self;\n`;
      pCtor += '}\n';
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

    let initWithManaged = `- (instancetype)${initWithManagedName}:(void*)${nameCamelBack}\n`;
    initWithManaged += '{\n';
    initWithManaged += `    NSAssert(${nameCamelBack} != nullptr && static_cast<std::shared_ptr<psm::${nameCxx}>*>(${nameCamelBack})->get() != nullptr, @"${nameCamelBack} is null");\n`;
    initWithManaged += `    self = [super init];\n`;
    initWithManaged += `    if (!self) {\n`;
    initWithManaged += `        return nil;\n`;
    initWithManaged += `    }\n`;
    initWithManaged += `    ${nameManagedMember} = std::move(*static_cast<std::shared_ptr<psm::${nameCxx}>*>(${nameCamelBack}));\n`;
    initWithManaged += `    return self;\n`;
    initWithManaged += '}\n';
    if (privateCtors.length > 0) {
      privateCtors += '\n';
    }
    privateCtors += initWithManaged;

    includesFirst.add('#include <utility>');

    let initWithNative = `- (instancetype)${initWithNativeName}:(void*)${nameCamelBack}\n`;
    initWithNative += '{\n';
    initWithNative += `    NSAssert(${nameCamelBack} != nullptr, @"${nameCamelBack} is null");\n`;
    initWithNative += `    self = [super init];\n`;
    initWithNative += `    if (!self) {\n`;
    initWithNative += `        return nil;\n`;
    initWithNative += `    }\n`;
    initWithNative += `    try {\n`;
    initWithNative += `        ${nameManagedMember}.reset(static_cast<psm::${nameCxx}*>(${nameCamelBack}));\n`;
    initWithNative += `    } catch (...) {\n`;
    initWithNative += `        return nil;\n`;
    initWithNative += `    }\n`;
    initWithNative += `    return self;\n`;
    initWithNative += '}\n';
    if (privateCtors.length > 0) {
      privateCtors += '\n';
    }
    privateCtors += initWithNative;
  }

  const copyConstructor = util.getClassCopyConstructor(interfaceJson);
  const cCtorDefinition = util.getConstructorDefinition(copyConstructor);
  const cCtorVisibility = util.getConstructorVisibility(copyConstructor);
  if (!static && copyable && cCtorDefinition !== util.ConstructorDefinition.deleted) {
    let cCtor = `- (instancetype)initWith${interfaceName}:(${name}*)${nameCamelBack}\n`;
    cCtor += `{\n`;
    cCtor += `    NSAssert(${nameCamelBack} != nil, @"${nameCamelBack} is null");\n`;
    cCtor += `    NSAssert(${nameCamelBack}->${nameManagedMember}.get() != nullptr, @"${nameCamelBack}->${nameManagedMember} is null");\n`;
    cCtor += `    auto* copy = new (std::nothrow) psm::${nameCxx}(*(${nameCamelBack}->${nameManagedMember}.get()));\n`;
    cCtor += `    if (!copy) {\n`;
    cCtor += `        return nil;\n`;
    cCtor += `    }\n`;
    cCtor += `    self = [self ${initWithNativeName}:copy];\n`;
    cCtor += `    if (!self) {\n`;
    cCtor += `        delete copy;\n`;
    cCtor += `        return nil;\n`;
    cCtor += `    }\n`;
    cCtor += `    return self;\n`;
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

    includesFirst.add('#include <new>');

    let copyWithZone = `- (instancetype)copyWithZone:(NSZone*)zone\n`;
    copyWithZone += `{\n`;
    copyWithZone += `    NSAssert(${nameManagedMember}.get() != nullptr, @"${nameManagedMember} is null");\n`;
    copyWithZone += `    auto* ${nameCamelBack} = new (std::nothrow) psm::${nameCxx}(*(${nameManagedMember}.get()));\n`;
    copyWithZone += `    if (!${nameCamelBack}) {\n`;
    copyWithZone += `        return nil;\n`;
    copyWithZone += `    }\n`;
    copyWithZone += `    ${name}* copy = [[[self class] allocWithZone:zone] ${initWithNativeName}:${nameCamelBack}];\n`;
    copyWithZone += `    if (!copy) {\n`;
    copyWithZone += `        delete ${nameCamelBack};\n`;
    copyWithZone += `        return nil;\n`;
    copyWithZone += `    }\n`;
    copyWithZone += `    return copy;\n`;
    copyWithZone += `}\n`;
    if (cCtorVisibility === util.Visibility.public) {
      if (publicCtors.length > 0) {
        publicCtors += '\n';
      }
      publicCtors += copyWithZone;
    } else {
      if (privateCtors.length > 0) {
        privateCtors += '\n';
      }
      privateCtors += copyWithZone;
    }

    includesFirst.add('#include <new>');
  }

  if (!static) {
    let dtor = `- (void)dealloc\n`;
    dtor += `{\n`;
    dtor += `    NSAssert(${nameManagedMember}.get() != nullptr, @"${nameManagedMember} is null");\n`;
    dtor += `}\n`;
    if (publicCtors.length > 0) {
      publicCtors += '\n';
    }
    publicCtors += dtor;
  }

  const equalityOperator = interfaceJson.equalityOperator;
  if (equalityOperator.defined) {
    let eqOp = `- (BOOL)isEqual:(id)object\n`;
    eqOp += `{\n`;
    eqOp += `    if (self == object) {\n`;
    eqOp += `        return YES;\n`;
    eqOp += `    }\n`;
    eqOp += `    if (![object isKindOfClass:[${name} class]]) {\n`;
    eqOp += `        return NO;\n`;
    eqOp += `    }\n`;
    eqOp += `    ${name}* ${nameCamelBack} = (${name}*)object;\n`;
    eqOp += `    NSAssert(${nameManagedMember}.get() != nullptr, @"${nameManagedMember} is null");\n`;
    eqOp += `    NSAssert(${nameCamelBack}->${nameManagedMember}.get() != nullptr, @"${nameCamelBack}->${nameManagedMember} is null");\n`;
    eqOp += `    return ${nameManagedMember}.get()->operator==(*(${nameCamelBack}->${nameManagedMember}.get()));\n`;
    eqOp += `}\n`;

    if (publicOperators.length > 0) {
      publicOperators += '\n';
    }
    publicOperators += eqOp;
  }

  const hashOperator = interfaceJson.hashOperator;
  if (hashOperator.defined) {
    let hashOp = `- (NSUInteger)hash\n`;
    hashOp += `{\n`;
    hashOp += `    NSAssert(${nameManagedMember}.get() != nullptr, @"${nameManagedMember} is null");\n`;
    hashOp += `    return std::hash<psm::${nameCxx}> {}(*(${nameManagedMember}.get()));\n`;
    hashOp += `}\n`;

    if (publicOperators.length > 0) {
      publicOperators += '\n';
    }
    publicOperators += hashOp;
  }

  for (const propertyJson of util.getProperties(interfaceJson)) {
    const propTypeRaw = propertyJson.type;
    const propStatic = util.getPropertyStatic(propertyJson);
    if (propertyJson.hasGetter) {
      const getterName = util.getPropertyGetterName(propertyJson, util.ObjC);
      const getterType = util.getPropertyTypeForGetter(interfaces, propertyJson, util.ObjC);
      let getterPfx = '';
      let getterExt = '';
      if (propTypeRaw === 'boolean') {
        getterExt = ' ? YES : NO';
      } else if (propTypeRaw === 'string' || propTypeRaw === 'string_ref' || propTypeRaw === 'string_mix') {
        getterPfx = '[NSString stringWithUTF8String:';
        getterExt = '.c_str()]';
      } else if (util.isClassOfAnyType(propTypeRaw)) {
        const propTypeRawWithoutPfx = propTypeRaw.split(':').slice(1).join(':');
        const propTypeInterfaceJson = interfaces[propTypeRawWithoutPfx];
        if (typeof propTypeInterfaceJson === 'undefined') {
          throw new Error(`Unknown class: ${propTypeRawWithoutPfx}`);
        }
        if (util.isClassType(propTypeRaw)) {
          getterPfx = `[[${getterType.substring(0, getterType.length - 1)} alloc] initWithNative${propTypeRawWithoutPfx}:new (std::nothrow) psm::${util.getLangClassName(propTypeInterfaceJson, util.CXX)}(std::move(`;
          getterExt = `))]`;
        } else if (util.isClassRefType(propTypeRaw) || util.isClassMixType(propTypeRaw)) {
          getterPfx = `[[${getterType.substring(0, getterType.length - 1)} alloc] initWithNative${propTypeRawWithoutPfx}:new (std::nothrow) psm::${util.getLangClassName(propTypeInterfaceJson, util.CXX)}(`;
          getterExt = `)]`;
        }
      }
      let getter = `${propStatic ? '+' : '-'} (${getterType})${getterName}\n`;
      getter += `{\n`;
      if (propStatic) {
        if (util.isClassPtrType(propTypeRaw) || util.isClassPtrRefType(propTypeRaw) || util.isClassPtrMixType(propTypeRaw)) {
          const propTypeRawWithoutPfx = propTypeRaw.split(':').slice(1).join(':');
          const propTypeInterfaceJson = interfaces[propTypeRawWithoutPfx];
          if (typeof propTypeInterfaceJson === 'undefined') {
            throw new Error(`Unknown class: ${propTypeRawWithoutPfx}`);
          }
          let movePfx = '', moveExt = '';
          if (util.isClassPtrType(propTypeRaw)) {
            movePfx = 'std::move(';
            moveExt = ')';
          }
          getter += `    std::shared_ptr<psm::${util.getLangClassName(propTypeInterfaceJson, util.CXX)}> getterResult = ${movePfx}${getterPfx}psm::${nameCxx}::${util.getPropertyGetterName(propertyJson, util.CXX)}()${getterExt}${moveExt};\n`;
          getter += `    return [[${getterType.substring(0, getterType.length - 1)} alloc] initWithManaged${propTypeRawWithoutPfx}:&getterResult];\n`;
        } else {
          getter += `    return ${getterPfx}psm::${nameCxx}::${util.getPropertyGetterName(propertyJson, util.CXX)}()${getterExt};\n`;
        }
      } else {
        getter += `    NSAssert(${nameManagedMember}.get() != nullptr, @"${nameManagedMember} is null");\n`;
        if (util.isClassPtrType(propTypeRaw) || util.isClassPtrRefType(propTypeRaw) || util.isClassPtrMixType(propTypeRaw)) {
          const propTypeRawWithoutPfx = propTypeRaw.split(':').slice(1).join(':');
          const propTypeInterfaceJson = interfaces[propTypeRawWithoutPfx];
          if (typeof propTypeInterfaceJson === 'undefined') {
            throw new Error(`Unknown class: ${propTypeRawWithoutPfx}`);
          }
          let movePfx = '', moveExt = '';
          if (util.isClassPtrType(propTypeRaw)) {
            movePfx = 'std::move(';
            moveExt = ')';
          }
          getter += `    std::shared_ptr<psm::${util.getLangClassName(propTypeInterfaceJson, util.CXX)}> getterResult = ${movePfx}${getterPfx}${nameManagedMember}.get()->${util.getPropertyGetterName(propertyJson, util.CXX)}()${getterExt}${moveExt};\n`;
          getter += `    return [[${getterType.substring(0, getterType.length - 1)} alloc] initWithManaged${propTypeRawWithoutPfx}:&getterResult];\n`;
        } else {
          getter += `    return ${getterPfx}${nameManagedMember}.get()->${util.getPropertyGetterName(propertyJson, util.CXX)}()${getterExt};\n`;
        }
      }
      getter += `}\n`;
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
        }
        if (util.isClassType(propTypeRaw) || util.isClassRefType(propTypeRaw) || util.isClassMixType(propTypeRaw)) {
          includesFirst.add('#include <new>');
        }
        if (util.isClassType(propTypeRaw) || util.isClassPtrType(propTypeRaw)) {
          includesFirst.add('#include <utility>');
        }
        if (util.isClassPtrType(propTypeRaw) || util.isClassPtrRefType(propTypeRaw) || util.isClassPtrMixType(propTypeRaw)) {
          includesFirst.add('#include <memory>');
        }
      }
    }
    if (propertyJson.hasSetter) {
      const setterName = util.getPropertySetterName(propertyJson, util.ObjC);
      const setterType = util.getPropertyTypeForSetter(interfaces, propertyJson, util.ObjC);
      const setterArgName = util.getPropertySetterArgName(propertyJson, util.ObjC);
      let setterPfx = '';
      let setterExt = '';
      if (propTypeRaw === 'boolean') {
        setterExt = ' ? true : false';
      } else if (propTypeRaw === 'string' || propTypeRaw === 'string_ref' || propTypeRaw === 'string_mix') {
        setterPfx = '[';
        setterExt = ' UTF8String]';
      } else if (util.isClassOfAnyType(propTypeRaw)) {
        const propTypeRawWithoutPfx = propTypeRaw.split(':').slice(1).join(':');
        const propTypeInterfaceJson = interfaces[propTypeRawWithoutPfx];
        if (typeof propTypeInterfaceJson === 'undefined') {
          throw new Error(`Unknown class: ${propTypeRawWithoutPfx}`);
        }
        if (util.isClassType(propTypeRaw) || util.isClassRefType(propTypeRaw) || util.isClassMixType(propTypeRaw)) {
          setterPfx = `*static_cast<const psm::${util.getLangClassName(propTypeInterfaceJson, util.CXX)}*>([`;
          setterExt = ` native${propTypeRawWithoutPfx}])`;
        } else if (util.isClassPtrType(propTypeRaw) || util.isClassPtrRefType(propTypeRaw) || util.isClassPtrMixType(propTypeRaw)) {
          setterPfx = `*static_cast<const std::shared_ptr<psm::${util.getLangClassName(propTypeInterfaceJson, util.CXX)}>*>([`;
          setterExt = ` managed${propTypeRawWithoutPfx}])`;
        }
      }
      let setter = `${propStatic ? '+' : '-'} (void)${setterName}:(${setterType})${setterArgName}\n`;
      setter += `{\n`;
      if (propStatic) {
        setter += `    psm::${nameCxx}::${util.getPropertySetterName(propertyJson, util.CXX)}(${setterPfx}${setterArgName}${setterExt});\n`;
      } else {
        setter += `    NSAssert(${nameManagedMember}.get() != nullptr, @"${nameManagedMember} is null");\n`;
        setter += `    ${nameManagedMember}.get()->${util.getPropertySetterName(propertyJson, util.CXX)}(${setterPfx}${setterArgName}${setterExt});\n`;
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
        }
        if (util.isClassPtrType(propTypeRaw) || util.isClassPtrRefType(propTypeRaw) || util.isClassPtrMixType(propTypeRaw)) {
          includesFirst.add('#include <memory>');
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
    code += '\n';
    code += `- (void*)${managedGetterName}\n`;
    code += `{\n`;
    code += `    NSAssert(${nameManagedMember}.get() != nullptr, @"${nameManagedMember} is null");\n`;
    code += `    return &${nameManagedMember};\n`;
    code += `}\n`;
    code += '\n';
    code += `- (void*)${nativeGetterName}\n`;
    code += `{\n`;
    code += `    NSAssert(${nameManagedMember}.get() != nullptr, @"${nameManagedMember} is null");\n`;
    code += `    return ${nameManagedMember}.get();\n`;
    code += `}\n`;
  }
  code += '\n';
  code += '@end\n';

  importsFirst = Array.from(importsFirst).sort();
  importsSecond = Array.from(importsSecond).sort();
  includesFirst = Array.from(includesFirst).sort();
  includesSecond = Array.from(includesSecond).sort();
  let includes = '';
  if (importsFirst.length > 0) {
    includes += '\n';
    for (const include of importsFirst) {
      includes += include + '\n';
    }
  }
  if (includesFirst.length > 0) {
    if (importsFirst.length > 0) {
      includes += '\n';
    }
    for (const include of includesFirst) {
      includes += include + '\n';
    }
  }
  if (importsSecond.length > 0) {
    includes += '\n';
    for (const include of importsSecond) {
      includes += include + '\n';
    }
  }
  if (includesSecond.length > 0) {
    if (importsSecond.length > 0) {
      includes += '\n';
    }
    for (const include of includesSecond) {
      includes += include + '\n';
    }
  }
  code = code.replaceAll('%INCLUDES%', includes);

  return code;
}

function generateInterfaceObjC(interfaces, interfaceName, interfaceJson) {
  const headerFilePath = path.resolve(__dirname, '..', 'platform', 'Apple', 'include', 'Posemesh', `${interfaceName}.h`);
  const sourceFilePath = path.resolve(__dirname, '..', 'platform', 'Apple', 'src', `${interfaceName}.mm`);

  let headerCode = generateHeader(interfaces, interfaceName, interfaceJson);
  let sourceCode = generateSource(interfaces, interfaceName, interfaceJson);

  util.writeFileContentIfDifferent(headerFilePath, headerCode);
  util.writeFileContentIfDifferent(sourceFilePath, sourceCode);
}

module.exports = generateInterfaceObjC;
