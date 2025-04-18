const path = require('path');
const util = require('./util');

function arrayGetterCode(enums, interfaces, propTypeRaw, getterType, nameCxx, nameManagedMember, propertyJson, propStatic) {
  const underlyingArrayTypeRaw = propTypeRaw.split(':').slice(1).join(':');
  let iteratedType = 'auto';
  if (underlyingArrayTypeRaw === 'string') {
    iteratedType = 'const std::string&';
  } else if (typeof interfaces[underlyingArrayTypeRaw] !== 'undefined' && util.isArrayType(propTypeRaw)) {
    iteratedType = `psm::${util.getLangClassName(interfaces[underlyingArrayTypeRaw], util.CXX)}`;
  } else if (typeof interfaces[underlyingArrayTypeRaw] !== 'undefined' && (util.isArrayRefType(propTypeRaw) || util.isArrayMixType(propTypeRaw))) {
    iteratedType = `const psm::${util.getLangClassName(interfaces[underlyingArrayTypeRaw], util.CXX)}&`;
  } else if (util.isArrayPtrType(propTypeRaw) || util.isArrayPtrRefType(propTypeRaw) || util.isArrayPtrMixType(propTypeRaw)) {
    iteratedType = `std::shared_ptr<psm::${util.getLangClassName(interfaces[underlyingArrayTypeRaw], util.CXX)}>`;
  }
  let getter = '';
  getter += `    NSMutable${getterType.substring(2/*!!!*/, getterType.length)} getterResult = [[NSMutableArray alloc] init];\n`;
  getter += `    for (${iteratedType} arrayElement : ${propStatic ? `psm::${nameCxx}::` : `${nameManagedMember}.get()->`}${util.getPropertyGetterName(propertyJson, util.CXX)}()) {\n`
  if (underlyingArrayTypeRaw === 'float') {
    getter += `        [getterResult addObject:[NSNumber numberWithFloat:arrayElement]];\n`;
  } else if (underlyingArrayTypeRaw === 'double') {
    getter += `        [getterResult addObject:[NSNumber numberWithDouble:arrayElement]];\n`;
  } else if (underlyingArrayTypeRaw === 'int8') {
    getter += `        [getterResult addObject:[NSNumber numberWithChar:arrayElement]];\n`;
  } else if (underlyingArrayTypeRaw === 'int16') {
    getter += `        [getterResult addObject:[NSNumber numberWithShort:arrayElement]];\n`;
  } else if (underlyingArrayTypeRaw === 'int32') {
    getter += `        [getterResult addObject:[NSNumber numberWithInt:arrayElement]];\n`;
  } else if (underlyingArrayTypeRaw === 'int64') {
    getter += `        [getterResult addObject:[NSNumber numberWithLongLong:arrayElement]];\n`;
  } else if (underlyingArrayTypeRaw === 'uint8') {
    getter += `        [getterResult addObject:[NSNumber numberWithUnsignedChar:arrayElement]];\n`;
  } else if (underlyingArrayTypeRaw === 'uint16') {
    getter += `        [getterResult addObject:[NSNumber numberWithUnsignedShort:arrayElement]];\n`;
  } else if (underlyingArrayTypeRaw === 'uint32') {
    getter += `        [getterResult addObject:[NSNumber numberWithUnsignedInt:arrayElement]];\n`;
  } else if (underlyingArrayTypeRaw === 'uint64') {
    getter += `        [getterResult addObject:[NSNumber numberWithUnsignedLongLong:arrayElement]];\n`;
  } else if (underlyingArrayTypeRaw === 'boolean') {
    getter += `        [getterResult addObject:[NSNumber numberWithBool:static_cast<BOOL>(arrayElement)]];\n`;
  } else if (underlyingArrayTypeRaw === 'string') {
    getter += `        [getterResult addObject:[NSString stringWithUTF8String:arrayElement.c_str()]];\n`;
  } else if (typeof enums[underlyingArrayTypeRaw] !== 'undefined') {
    if (enums[underlyingArrayTypeRaw].type === 'flag') {
      getter += `        [getterResult addObject:[NSNumber numberWithUnsignedInteger:static_cast<NSUInteger>(arrayElement)]];\n`;
    } else {
      getter += `        [getterResult addObject:[NSNumber numberWithInteger:static_cast<NSInteger>(arrayElement)]];\n`;
    }
  } else if (typeof interfaces[underlyingArrayTypeRaw] !== 'undefined') {
    if (util.isArrayType(propTypeRaw)) {
      getter += `        [getterResult addObject:[[${getterType.substring(8/*!!!*/, getterType.length - 3)} alloc] initWithNative${underlyingArrayTypeRaw}:new (std::nothrow) psm::${util.getLangClassName(interfaces[underlyingArrayTypeRaw], util.CXX)}(std::move(arrayElement))]];\n`;
    } else if (util.isArrayRefType(propTypeRaw) || util.isArrayMixType(propTypeRaw)) {
      getter += `        [getterResult addObject:[[${getterType.substring(8/*!!!*/, getterType.length - 3)} alloc] initWithNative${underlyingArrayTypeRaw}:new (std::nothrow) psm::${util.getLangClassName(interfaces[underlyingArrayTypeRaw], util.CXX)}(arrayElement)]];\n`;
    } else {
      getter += `        if (arrayElement) {\n`;
      getter += `            [getterResult addObject:[[${util.getLangClassName(interfaces[underlyingArrayTypeRaw], util.ObjC)} alloc] initWithManaged${underlyingArrayTypeRaw}:&arrayElement]];\n`;
      getter += `        } else {\n`;
      getter += `            [getterResult addObject:[NSNull null]];\n`;
      getter += `        }\n`;
    }
  } else {
    throw new Error(`Unhandled type: ${propTypeRaw}`);
  }
  getter += `    }\n`
  getter += `    return getterResult;\n`
  return getter;
}

function arraySetterCode(enums, interfaces, propTypeRaw, setterType, setterArgName, nameCxx, nameManagedMember, propertyJson, propStatic) {
  const underlyingArrayTypeRaw = propTypeRaw.split(':').slice(1).join(':');
  let setter = '';
  if (!propStatic) {
    setter += `    NSAssert(${nameManagedMember}.get() != nullptr, @"${nameManagedMember} is null");\n`;
  }
  const isArrayOfNativeClasses = typeof interfaces[underlyingArrayTypeRaw] !== 'undefined' && (util.isArrayType(propTypeRaw) || util.isArrayRefType(propTypeRaw) || util.isArrayMixType(propTypeRaw));
  if (underlyingArrayTypeRaw !== 'string' && !isArrayOfNativeClasses) {
    setter += `    for (${setterType.substring(8/*!!!*/, setterType.length - 2)} arrayElement in ${setterArgName}) {\n`;
    if (underlyingArrayTypeRaw === 'float') {
      setter += `        NSAssert(std::strcmp([arrayElement objCType], @encode(float)) == 0, @"${setterArgName} contains at least one invalid element type");\n`;
    } else if (underlyingArrayTypeRaw === 'double') {
      setter += `        NSAssert(std::strcmp([arrayElement objCType], @encode(double)) == 0, @"${setterArgName} contains at least one invalid element type");\n`;
    } else if (underlyingArrayTypeRaw === 'int8') {
      setter += `        NSAssert(std::strcmp([arrayElement objCType], @encode(char)) == 0, @"${setterArgName} contains at least one invalid element type");\n`;
    } else if (underlyingArrayTypeRaw === 'int16') {
      setter += `        NSAssert(std::strcmp([arrayElement objCType], @encode(short)) == 0, @"${setterArgName} contains at least one invalid element type");\n`;
    } else if (underlyingArrayTypeRaw === 'int32') {
      setter += `        NSAssert(std::strcmp([arrayElement objCType], @encode(int)) == 0, @"${setterArgName} contains at least one invalid element type");\n`;
    } else if (underlyingArrayTypeRaw === 'int64') {
      setter += `        NSAssert(std::strcmp([arrayElement objCType], @encode(long long)) == 0, @"${setterArgName} contains at least one invalid element type");\n`;
    } else if (underlyingArrayTypeRaw === 'uint8') {
      setter += `        NSAssert(std::strcmp([arrayElement objCType], @encode(unsigned char)) == 0, @"${setterArgName} contains at least one invalid element type");\n`;
    } else if (underlyingArrayTypeRaw === 'uint16') {
      setter += `        NSAssert(std::strcmp([arrayElement objCType], @encode(unsigned short)) == 0, @"${setterArgName} contains at least one invalid element type");\n`;
    } else if (underlyingArrayTypeRaw === 'uint32') {
      setter += `        NSAssert(std::strcmp([arrayElement objCType], @encode(unsigned int)) == 0, @"${setterArgName} contains at least one invalid element type");\n`;
    } else if (underlyingArrayTypeRaw === 'uint64') {
      setter += `        NSAssert(std::strcmp([arrayElement objCType], @encode(unsigned long long)) == 0, @"${setterArgName} contains at least one invalid element type");\n`;
    } else if (underlyingArrayTypeRaw === 'boolean') {
      setter += `        NSAssert(std::strcmp([arrayElement objCType], @encode(BOOL)) == 0, @"${setterArgName} contains at least one invalid element type");\n`;
    } else if (typeof enums[underlyingArrayTypeRaw] !== 'undefined') {
      if (enums[underlyingArrayTypeRaw].type === 'flag') {
        setter += `        NSAssert(std::strcmp([arrayElement objCType], @encode(NSUInteger)) == 0 || std::strcmp([arrayElement objCType], @encode(unsigned int)) == 0, @"${setterArgName} contains at least one invalid element type");\n`;
      } else {
        setter += `        NSAssert(std::strcmp([arrayElement objCType], @encode(NSInteger)) == 0 || std::strcmp([arrayElement objCType], @encode(int)) == 0, @"${setterArgName} contains at least one invalid element type");\n`;
      }
    } else if (typeof interfaces[underlyingArrayTypeRaw] !== 'undefined') {
      if (util.isArrayPtrType(propTypeRaw) || util.isArrayPtrRefType(propTypeRaw) || util.isArrayPtrMixType(propTypeRaw)) {
        setter += `        NSAssert([arrayElement isKindOfClass:[${util.getLangClassName(interfaces[underlyingArrayTypeRaw], util.ObjC)} class]] || [arrayElement isKindOfClass:[NSNull class]], @"${setterArgName} contains at least one invalid element type");\n`;
      } else {
        // Unreachable branch.
      }
    } else {
      throw new Error(`Unhandled type: ${propTypeRaw}`);
    }
    setter += `    }\n`;
  }
  if (typeof enums[underlyingArrayTypeRaw] !== 'undefined') {
    setter += `    std::vector<psm::${util.getLangEnumName(enums[underlyingArrayTypeRaw], util.CXX)}> temporaryVector;\n`;
  } else if (typeof interfaces[underlyingArrayTypeRaw] !== 'undefined') {
    if (util.isArrayType(propTypeRaw) || util.isArrayRefType(propTypeRaw) || util.isArrayMixType(propTypeRaw)) {
      setter += `    std::vector<psm::${util.getLangClassName(interfaces[underlyingArrayTypeRaw], util.CXX)}> temporaryVector;\n`;
    } else {
      setter += `    std::vector<std::shared_ptr<psm::${util.getLangClassName(interfaces[underlyingArrayTypeRaw], util.CXX)}>> temporaryVector;\n`;
    }
  } else {
    setter += `    ${util.getPropertyType(enums, interfaces, propertyJson, util.CXX)} temporaryVector;\n`;
  }
  setter += `    temporaryVector.reserve([${setterArgName} count]);\n`;
  setter += `    for (${setterType.substring(8/*!!!*/, setterType.length - 2)} arrayElement in ${setterArgName}) {\n`;
  if (underlyingArrayTypeRaw === 'float') {
    setter += `        temporaryVector.push_back([arrayElement floatValue]);\n`;
  } else if (underlyingArrayTypeRaw === 'double') {
    setter += `        temporaryVector.push_back([arrayElement doubleValue]);\n`;
  } else if (underlyingArrayTypeRaw === 'int8') {
    setter += `        temporaryVector.push_back([arrayElement charValue]);\n`;
  } else if (underlyingArrayTypeRaw === 'int16') {
    setter += `        temporaryVector.push_back([arrayElement shortValue]);\n`;
  } else if (underlyingArrayTypeRaw === 'int32') {
    setter += `        temporaryVector.push_back([arrayElement intValue]);\n`;
  } else if (underlyingArrayTypeRaw === 'int64') {
    setter += `        temporaryVector.push_back([arrayElement longLongValue]);\n`;
  } else if (underlyingArrayTypeRaw === 'uint8') {
    setter += `        temporaryVector.push_back([arrayElement unsignedCharValue]);\n`;
  } else if (underlyingArrayTypeRaw === 'uint16') {
    setter += `        temporaryVector.push_back([arrayElement unsignedShortValue]);\n`;
  } else if (underlyingArrayTypeRaw === 'uint32') {
    setter += `        temporaryVector.push_back([arrayElement unsignedIntValue]);\n`;
  } else if (underlyingArrayTypeRaw === 'uint64') {
    setter += `        temporaryVector.push_back([arrayElement unsignedLongLongValue]);\n`;
  } else if (underlyingArrayTypeRaw === 'boolean') {
    setter += `        temporaryVector.push_back(static_cast<bool>([arrayElement boolValue]));\n`;
  } else if (underlyingArrayTypeRaw === 'string') {
    setter += `        temporaryVector.emplace_back([arrayElement UTF8String]);\n`;
  } else if (typeof enums[underlyingArrayTypeRaw] !== 'undefined') {
    if (enums[underlyingArrayTypeRaw].type === 'flag') {
      setter += `        if (std::strcmp([arrayElement objCType], @encode(NSUInteger)) == 0) {\n`;
      setter += `            temporaryVector.push_back(static_cast<decltype(temporaryVector)::value_type>([arrayElement unsignedIntegerValue]));\n`;
      setter += `        } else {\n`;
      setter += `            temporaryVector.push_back(static_cast<decltype(temporaryVector)::value_type>([arrayElement unsignedIntValue]));\n`;
      setter += `        }\n`;
    } else {
      setter += `        if (std::strcmp([arrayElement objCType], @encode(NSInteger)) == 0) {\n`;
      setter += `            temporaryVector.push_back(static_cast<decltype(temporaryVector)::value_type>([arrayElement integerValue]));\n`;
      setter += `        } else {\n`;
      setter += `            temporaryVector.push_back(static_cast<decltype(temporaryVector)::value_type>([arrayElement intValue]));\n`;
      setter += `        }\n`;
    }
  } else if (typeof interfaces[underlyingArrayTypeRaw] !== 'undefined') {
    if (util.isArrayType(propTypeRaw) || util.isArrayRefType(propTypeRaw) || util.isArrayMixType(propTypeRaw)) {
      setter += `        temporaryVector.emplace_back(*static_cast<psm::${util.getLangClassName(interfaces[underlyingArrayTypeRaw], util.CXX)}*>([arrayElement native${underlyingArrayTypeRaw}]));\n`;
    } else {
      setter += `        temporaryVector.emplace_back([arrayElement isKindOfClass:[${util.getLangClassName(interfaces[underlyingArrayTypeRaw], util.ObjC)} class]] ? *static_cast<std::shared_ptr<psm::${util.getLangClassName(interfaces[underlyingArrayTypeRaw], util.CXX)}>*>([((${util.getLangClassName(interfaces[underlyingArrayTypeRaw], util.ObjC)}*)arrayElement) managed${underlyingArrayTypeRaw}]) : nullptr);\n`;
    }
  } else {
    throw new Error(`Unhandled type: ${propTypeRaw}`);
  }
  setter += `    }\n`;
  if (util.isArrayType(propTypeRaw) || util.isArrayMixType(propTypeRaw) || util.isArrayPtrType(propTypeRaw) || util.isArrayPtrMixType(propTypeRaw)) {
    setter += `    ${propStatic ? `psm::${nameCxx}::` : `${nameManagedMember}.get()->`}${util.getPropertySetterName(propertyJson, util.CXX)}(std::move(temporaryVector));\n`;
  } else {
    setter += `    ${propStatic ? `psm::${nameCxx}::` : `${nameManagedMember}.get()->`}${util.getPropertySetterName(propertyJson, util.CXX)}(temporaryVector);\n`;
  }
  return setter;
}

function generateHeader(enums, interfaces, interfaceName, interfaceJson) {
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
    publicCtors += '- (instancetype _Nonnull)init NS_UNAVAILABLE;\n';
  } else {
    publicCtors += '- (instancetype _Nonnull)init;\n';
  }

  const copyConstructor = util.getClassCopyConstructor(interfaceJson);
  const cCtorDefinition = util.getConstructorDefinition(copyConstructor);
  const cCtorVisibility = util.getConstructorVisibility(copyConstructor);
  if (!static && copyable && cCtorDefinition !== util.ConstructorDefinition.deleted && cCtorVisibility === util.Visibility.public) {
    publicCtors += `- (instancetype _Nonnull)initWith${interfaceName}:(${name}* _Nonnull)${nameCamelBack};\n`;
    publicCtors += `- (instancetype _Nonnull)copyWithZone:(NSZone* _Null_unspecified)zone;\n`;
  } else {
    publicCtors += `- (instancetype _Nonnull)copy NS_UNAVAILABLE;\n`;
  }

  if (!static) {
    publicCtors += `- (void)dealloc;\n`;
  }

  const equalityOperator = interfaceJson.equalityOperator;
  if (equalityOperator.defined) {
    publicOperators += `- (BOOL)isEqual:(id _Null_unspecified)object;\n`;
  }

  const hashOperator = interfaceJson.hashOperator;
  if (hashOperator.defined) {
    publicOperators += `- (NSUInteger)hash;\n`;
  }

  const toStringOperator = interfaceJson.toStringOperator;
  if (toStringOperator.defined) {
    publicOperators += `- (NSString* _Nonnull)description;\n`;
  }

  for (const propertyJson of util.getProperties(interfaceJson)) {
    let shouldAddIncludes = false;
    const propStatic = util.getPropertyStatic(propertyJson);
    if (propertyJson.hasGetter) {
      const getterName = util.getPropertyGetterName(propertyJson, util.ObjC);
      const getterType = util.getPropertyTypeForGetter(enums, interfaces, propertyJson, util.ObjC);
      let getterTypeExt = '';
      if (propertyJson.type === 'string' || propertyJson.type === 'string_ref' || propertyJson.type === 'string_mix' || util.isClassOfAnyType(propertyJson.type) || util.isArrayOfAnyType(propertyJson.type)) {
        if (util.isClassPtrType(propertyJson.type) || util.isClassPtrRefType(propertyJson.type) || util.isClassPtrMixType(propertyJson.type)) {
          getterTypeExt = ' _Nullable';
        } else {
          getterTypeExt = ' _Nonnull';
        }
      }
      const getter = `${propStatic ? '+' : '-'} (${getterType}${getterTypeExt})${getterName} NS_REFINED_FOR_SWIFT;\n`;
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
      const setterType = util.getPropertyTypeForSetter(enums, interfaces, propertyJson, util.ObjC);
      let setterTypeExt = '';
      if (propertyJson.type === 'string' || propertyJson.type === 'string_ref' || propertyJson.type === 'string_mix' || util.isClassOfAnyType(propertyJson.type) || util.isArrayOfAnyType(propertyJson.type)) {
        if (util.isClassPtrType(propertyJson.type) || util.isClassPtrRefType(propertyJson.type) || util.isClassPtrMixType(propertyJson.type)) {
          setterTypeExt = ' _Nullable';
        } else {
          setterTypeExt = ' _Nonnull';
        }
      }
      const setterArgName = util.getPropertySetterArgName(propertyJson, util.ObjC);
      const setter = `${propStatic ? '+' : '-'} (void)${setterName}:(${setterType}${setterTypeExt})${setterArgName} NS_REFINED_FOR_SWIFT;\n`;
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
      } else if (util.isEnumType(propTypeRaw)) {
        importsSecond.add(`#import "${propTypeRaw.split(':').slice(1).join(':')}.h"`);
      } else if (util.isClassOfAnyType(propTypeRaw)) {
        importsSecond.add(`#import "${propTypeRaw.split(':').slice(1).join(':')}.h"`);
      } else if (util.isArrayOfAnyType(propTypeRaw)) {
        const underlyingArrayTypeRaw = propTypeRaw.split(':').slice(1).join(':');
        if (typeof enums[underlyingArrayTypeRaw] !== 'undefined' || typeof interfaces[underlyingArrayTypeRaw] !== 'undefined') {
          importsSecond.add(`#import "${underlyingArrayTypeRaw}.h"`);
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

  if (public.length > 0) {
    code += '\n';
    code += public;
  }

  if (!static) {
    code += '\n';
    code += '#if defined(POSEMESH_BUILD)\n';
    code += `- (instancetype _Nonnull)initWithManaged${interfaceName}:(void* _Nonnull)${nameCamelBack};\n`;
    code += `- (instancetype _Nonnull)initWithNative${interfaceName}:(void* _Nonnull)${nameCamelBack};\n`;
    code += `- (void* _Nonnull)${managedGetterName};\n`;
    code += `- (void* _Nonnull)${nativeGetterName};\n`;
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

function generateSource(enums, interfaces, interfaceName, interfaceJson) {
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

  const toStringOperator = interfaceJson.toStringOperator;
  if (toStringOperator.defined) {
    let toStrOp = `- (NSString*)description\n`;
    toStrOp += `{\n`;
    toStrOp += `    NSAssert(${nameManagedMember}.get() != nullptr, @"${nameManagedMember} is null");\n`;
    toStrOp += `    return [NSString stringWithUTF8String:static_cast<std::string>(*(${nameManagedMember}.get())).c_str()];\n`;
    toStrOp += `}\n`;

    if (publicOperators.length > 0) {
      publicOperators += '\n';
    }
    publicOperators += toStrOp;
  }

  for (const propertyJson of util.getProperties(interfaceJson)) {
    const propTypeRaw = propertyJson.type;
    const propStatic = util.getPropertyStatic(propertyJson);
    if (propertyJson.hasGetter) {
      const getterName = util.getPropertyGetterName(propertyJson, util.ObjC);
      const getterType = util.getPropertyTypeForGetter(enums, interfaces, propertyJson, util.ObjC);
      let getterPfx = '';
      let getterExt = '';
      if (propTypeRaw === 'boolean') {
        getterExt = ' ? YES : NO';
      } else if (propTypeRaw === 'string' || propTypeRaw === 'string_ref' || propTypeRaw === 'string_mix') {
        getterPfx = '[NSString stringWithUTF8String:';
        getterExt = '.c_str()]';
      } else if (util.isEnumType(propTypeRaw)) {
        getterPfx = `static_cast<${getterType}>(`;
        getterExt = ')';
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
          getter += `    return getterResult ? [[${getterType.substring(0, getterType.length - 1)} alloc] initWithManaged${propTypeRawWithoutPfx}:&getterResult] : nil;\n`;
        } else if (util.isArrayOfAnyType(propTypeRaw)) {
          getter += arrayGetterCode(enums, interfaces, propTypeRaw, getterType, nameCxx, nameManagedMember, propertyJson, propStatic);
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
          getter += `    return getterResult ? [[${getterType.substring(0, getterType.length - 1)} alloc] initWithManaged${propTypeRawWithoutPfx}:&getterResult] : nil;\n`;
        } else if (util.isArrayOfAnyType(propTypeRaw)) {
          getter += arrayGetterCode(enums, interfaces, propTypeRaw, getterType, nameCxx, nameManagedMember, propertyJson, propStatic);
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
        if (util.isArrayOfAnyType(propTypeRaw)) {
          const underlyingArrayTypeRaw = propTypeRaw.split(':').slice(1).join(':');
          if (typeof interfaces[underlyingArrayTypeRaw] !== 'undefined') {
            if (util.isArrayType(propTypeRaw)) {
              includesFirst.add('#include <new>');
              includesFirst.add('#include <utility>');
            } else if (util.isArrayRefType(propTypeRaw) || util.isArrayMixType(propTypeRaw)) {
              includesFirst.add('#include <new>');
            }
          }
        }
      }
    }
    if (propertyJson.hasSetter) {
      const setterName = util.getPropertySetterName(propertyJson, util.ObjC);
      const setterType = util.getPropertyTypeForSetter(enums, interfaces, propertyJson, util.ObjC);
      const setterArgName = util.getPropertySetterArgName(propertyJson, util.ObjC);
      let setterPfx = '';
      let setterExt = '';
      if (propTypeRaw === 'boolean') {
        setterExt = ' ? true : false';
      } else if (propTypeRaw === 'string' || propTypeRaw === 'string_ref' || propTypeRaw === 'string_mix') {
        setterPfx = '[';
        setterExt = ' UTF8String]';
      } else if (util.isEnumType(propTypeRaw)) {
        setterPfx = `static_cast<psm::${util.getPropertyType(enums, interfaces, propertyJson, util.CXX)}>(`;
        setterExt = ')';
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
          setterPfx = `${setterArgName} ? *static_cast<const std::shared_ptr<psm::${util.getLangClassName(propTypeInterfaceJson, util.CXX)}>*>([`;
          setterExt = ` managed${propTypeRawWithoutPfx}]) : std::shared_ptr<psm::${util.getLangClassName(propTypeInterfaceJson, util.CXX)}> {}`;
        }
      }
      let setter = `${propStatic ? '+' : '-'} (void)${setterName}:(${setterType})${setterArgName}\n`;
      setter += `{\n`;
      if (util.isArrayOfAnyType(propTypeRaw)) {
        setter += arraySetterCode(enums, interfaces, propTypeRaw, setterType, setterArgName, nameCxx, nameManagedMember, propertyJson, propStatic);
      } else if (propStatic) {
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
        if (util.isArrayOfAnyType(propTypeRaw)) {
          const underlyingArrayTypeRaw = propTypeRaw.split(':').slice(1).join(':');
          const isArrayOfNativeClasses = typeof interfaces[underlyingArrayTypeRaw] !== 'undefined' && (util.isArrayType(propTypeRaw) || util.isArrayRefType(propTypeRaw) || util.isArrayMixType(propTypeRaw));
          if (underlyingArrayTypeRaw !== 'string' && !isArrayOfNativeClasses) {
            includesFirst.add('#include <cstring>');
          }
          if (util.isArrayType(propTypeRaw) || util.isArrayMixType(propTypeRaw) || util.isArrayPtrType(propTypeRaw) || util.isArrayPtrMixType(propTypeRaw)) {
            includesFirst.add('#include <utility>');
          }
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

function generateInterfaceObjC(enums, interfaces, interfaceName, interfaceJson) {
  const headerFilePath = path.resolve(__dirname, '..', 'platform', 'Apple', 'include', 'Posemesh', `${interfaceName}.h`);
  const sourceFilePath = path.resolve(__dirname, '..', 'platform', 'Apple', 'src', `${interfaceName}.mm`);

  let headerCode = generateHeader(enums, interfaces, interfaceName, interfaceJson);
  let sourceCode = generateSource(enums, interfaces, interfaceName, interfaceJson);

  util.writeFileContentIfDifferent(headerFilePath, headerCode);
  util.writeFileContentIfDifferent(sourceFilePath, sourceCode);
}

module.exports = generateInterfaceObjC;
