const fs = require('fs');

const NameStyle = {
  lower_case: 1,
  UPPER_CASE: 2,
  camelBack: 3,
  CamelCase: 4,
  camel_Snake_Back: 5,
  Camel_Snake_Case: 6,
  Leading_upper_snake_case: 7
};

const NameStylePretty = {
  lowerCase: NameStyle.lower_case,
  upperCase: NameStyle.UPPER_CASE,
  camelBack: NameStyle.camelBack,
  camelCase: NameStyle.CamelCase,
  camelSnakeBack: NameStyle.camel_Snake_Back,
  camelSnakeCase: NameStyle.Camel_Snake_Case,
  leadingUpperSnakeCase: NameStyle.Leading_upper_snake_case
};

const Language = {
  CXX: 1,
  C: 2,
  ObjC: 3,
  Swift: 4,
  JS: 5
};

const LanguagePretty = {
  cPlusPlus: Language.CXX,
  c: Language.C,
  objectiveC: Language.ObjC,
  swift: Language.Swift,
  javaScript: Language.JS
};

const MethodMode = {
  regular: 'regular',
  virtual: 'virtual',
  pureVirtual: 'pureVirtual',
  override: 'override'
};

const Visibility = {
  public: 'public',
  protected: 'protected',
  private: 'private'
};

const ConstructorDefinition = {
  defined: 'defined',
  default: 'default',
  deleted: 'deleted',
  omitted: 'omitted'
};

const DestructorDefinition = {
  defined: 'defined',
  default: 'default',
  omitted: 'omitted'
};

function getName(key, json) {
  if (typeof json[key] === 'undefined') {
    throw new Error(`Missing '${key}' key.`);
  }
  if (typeof json[key] !== 'string') {
    throw new Error(`Invalid '${key}' key type.`);
  }
  return json[key];
}

function getStyleName(key, json, nameStyle) {
  for (const [styleKey, styleValue] of Object.entries(NameStylePretty)) {
    if (styleValue === nameStyle) {
      return getName(`${key}.style.${styleKey}`, json);
    }
  }
  throw new Error(`Unknown name style: ${nameStyle}`);
}

function getLangName(key, json, language) {
  for (const [langKey, langValue] of Object.entries(LanguagePretty)) {
    if (langValue === language) {
      return getName(`${key}.lang.${langKey}`, json);
    }
  }
  throw new Error(`Unknown language: ${language}`);
}

let defaultEnumNameLangToTransformationMap = {};
defaultEnumNameLangToTransformationMap[Language.CXX] = '%';
defaultEnumNameLangToTransformationMap[Language.C] = 'psm_%_e';
defaultEnumNameLangToTransformationMap[Language.ObjC] = 'PSM%';
defaultEnumNameLangToTransformationMap[Language.Swift] = '%';
defaultEnumNameLangToTransformationMap[Language.JS] = '%';

let defaultEnumConstantNameLangToTransformationMap = {};
defaultEnumConstantNameLangToTransformationMap[Language.CXX] = '%';
defaultEnumConstantNameLangToTransformationMap[Language.C] = '%';
defaultEnumConstantNameLangToTransformationMap[Language.ObjC] = '%';
defaultEnumConstantNameLangToTransformationMap[Language.Swift] = '%'; // don't care
defaultEnumConstantNameLangToTransformationMap[Language.JS] = '%';

let defaultClassNameLangToTransformationMap = {};
defaultClassNameLangToTransformationMap[Language.CXX] = '%';
defaultClassNameLangToTransformationMap[Language.C] = 'psm_%_t';
defaultClassNameLangToTransformationMap[Language.ObjC] = 'PSM%';
defaultClassNameLangToTransformationMap[Language.Swift] = '%';
defaultClassNameLangToTransformationMap[Language.JS] = '%';

function getLangTransformedName(key, json, language, nameLangToTransformationMap) {
  const transformation = nameLangToTransformationMap[language];
  if (typeof transformation === 'undefined') {
    throw new Error(`Unknown language: ${language}`);
  }
  const parts = transformation.split('%');
  if (parts.length !== 2) {
    throw new Error('Invalid transformation.');
  }
  return parts[0] + getLangName(key, json, language) + parts[1];
}

function getLangEnumName(enumJson, language, nameLangToTransformationMap = defaultEnumNameLangToTransformationMap) {
  return getLangTransformedName('name', enumJson, language, nameLangToTransformationMap);
}

function getLangEnumConstantName(constantJson, language, nameLangToTransformationMap = defaultEnumConstantNameLangToTransformationMap) {
  return getLangTransformedName('name', constantJson, language, nameLangToTransformationMap);
}

function getLangClassName(interfaceJson, language, nameLangToTransformationMap = defaultClassNameLangToTransformationMap) {
  return getLangTransformedName('name', interfaceJson, language, nameLangToTransformationMap);
}

function getClassStatic(interfaceJson) {
  return interfaceJson.static;
}

function getClassFinal(interfaceJson) {
  return interfaceJson.final;
}

function getClassCopyable(interfaceJson) {
  return interfaceJson.copyable;
}

function getClassMovable(interfaceJson) {
  return interfaceJson.movable;
}

function getClassParameterlessConstructor(interfaceJson) {
  return interfaceJson.parameterlessConstructor;
}

function getClassCopyConstructor(interfaceJson) {
  return interfaceJson.copyConstructor;
}

function getClassMoveConstructor(interfaceJson) {
  return interfaceJson.moveConstructor;
}

function getConstructorDefinition(constructorJson) {
  return constructorJson.definition;
}

function getConstructorVisibility(constructorJson) {
  return constructorJson.visibility;
}

function getConstructorNoexcept(constructorJson) {
  return constructorJson.noexcept;
}

let defaultFuncArgNameLangToTransformationMap = {};
defaultFuncArgNameLangToTransformationMap[Language.CXX] = '%';
defaultFuncArgNameLangToTransformationMap[Language.C] = '%';
defaultFuncArgNameLangToTransformationMap[Language.ObjC] = '%';
defaultFuncArgNameLangToTransformationMap[Language.Swift] = '%';
defaultFuncArgNameLangToTransformationMap[Language.JS] = '%';

function getCopyOrMoveConstructorMainArgName(constructorJson, language, funcArgNameLangToTransformationMap = defaultFuncArgNameLangToTransformationMap) {
  return getLangTransformedName('mainArgName', constructorJson, language, funcArgNameLangToTransformationMap);
}

function getConstructorInitializedProperties(constructorJson) {
  return constructorJson.initializedProperties;
}

function getConstructorCodeFront(constructorJson) {
  return constructorJson.codeFront;
}

function getConstructorCodeBack(constructorJson) {
  return constructorJson.codeBack;
}

function getConstructorOperatorCodeFront(constructorJson) {
  return constructorJson.operatorCodeFront;
}

function getConstructorOperatorCodeBack(constructorJson) {
  return constructorJson.operatorCodeBack;
}

function getConstructorCustom(constructorJson) {
  return constructorJson.custom;
}

function getConstructorCustomOperator(constructorJson) {
  return constructorJson.customOperator;
}

function getDestructorVirtual(destructorJson) {
  return destructorJson.virtual;
}

function getDestructorCode(destructorJson) {
  return destructorJson.code;
}

function getDestructorDefinition(destructorJson) {
  return destructorJson.definition;
}

function getDestructorVisibility(destructorJson) {
  return destructorJson.visibility;
}

function getDestructorCustom(destructorJson) {
  return destructorJson.custom;
}

function getLangAliases(interfaceJson, language, nameLangToTransformationMap = defaultClassNameLangToTransformationMap) {
  if (typeof interfaceJson.aliases === 'undefined') {
    throw new Error(`Missing 'aliases' key.`);
  }
  let aliases = [];
  for (const aliasJson of interfaceJson.aliases) {
    const alias = getLangClassName(aliasJson, language, nameLangToTransformationMap);
    aliases.push(alias);
  }
  return aliases;
}

function getHeaderGuardName(interfaceJson) {
  return getName('headerGuardName', interfaceJson);
}

let defaultPropNameLangToTransformationMap = {};
defaultPropNameLangToTransformationMap[Language.CXX] = 'm_%';
defaultPropNameLangToTransformationMap[Language.C] = '%'; // don't care
defaultPropNameLangToTransformationMap[Language.ObjC] = '%'; // don't care
defaultPropNameLangToTransformationMap[Language.Swift] = '%';
defaultPropNameLangToTransformationMap[Language.JS] = '%';

function getPropertyName(propertyJson, language, nameLangToTransformationMap = defaultPropNameLangToTransformationMap) {
  return getLangTransformedName('name', propertyJson, language, nameLangToTransformationMap);
}

function getFloatType(language) {
  switch (language) {
    case Language.CXX:
    case Language.C:
    case Language.ObjC:
      return 'float';
    case Language.Swift:
      return 'Float';
    case Language.JS:
      return 'number';
    default:
      throw new Error(`Unknown language: ${language}`);
  }
}

function getDoubleType(language) {
  switch (language) {
    case Language.CXX:
    case Language.C:
    case Language.ObjC:
      return 'double';
    case Language.Swift:
      return 'Double';
    case Language.JS:
      return 'number';
    default:
      throw new Error(`Unknown language: ${language}`);
  }
}

function getIntType(signed, bits, language) {
  switch (bits) {
    case '8':
    case '16':
    case '32':
    case '64':
      break;
    default:
      throw new Error(`Invalid integer bits: ${bits}`);
  }
  switch (language) {
    case Language.CXX:
      return `std::${signed ? '' : 'u'}int${bits}_t`;
    case Language.C:
    case Language.ObjC:
      return `${signed ? '' : 'u'}int${bits}_t`;
    case Language.Swift:
      return `${signed ? '' : 'U'}Int${bits}`;
    case Language.JS:
      if (bits === '64')
        return 'bigint';
      return 'number';
    default:
      throw new Error(`Unknown language: ${language}`);
  }
}

function getBooleanType(language) {
  switch (language) {
    case Language.CXX:
      return 'bool';
    case Language.C:
      return 'uint8_t';
    case Language.ObjC:
      return 'BOOL';
    case Language.Swift:
      return 'Bool';
    case Language.JS:
      return 'boolean';
    default:
      throw new Error(`Unknown language: ${language}`);
  }
}

function getStringType(language) {
  switch (language) {
    case Language.CXX:
      return 'std::string';
    case Language.C:
      return 'const char*';
    case Language.ObjC:
      return 'NSString*';
    case Language.Swift:
      return 'String';
    case Language.JS:
      return 'string';
    default:
      throw new Error(`Unknown language: ${language}`);
  }
}

const TypeFor = {
  Any: 0,
  PropGetter: 1,
  PropSetter: 2
};

function getStringRefType(language, typeFor = TypeFor.Any) {
  if (language === Language.CXX) {
    switch (typeFor) {
      case TypeFor.Any:
        return 'std::string';
      case TypeFor.PropGetter:
      case TypeFor.PropSetter:
        return 'const std::string&';
      default:
        throw new Error(`Unknown TypeFor value: ${typeFor}`);
    }
  }
  return getStringType(language);
}

function getStringMixType(language, typeFor = TypeFor.Any) {
  if (language === Language.CXX) {
    switch (typeFor) {
      case TypeFor.Any:
        return 'std::string';
      case TypeFor.PropGetter:
        return 'const std::string&';
      case TypeFor.PropSetter:
        return 'std::string';
      default:
        throw new Error(`Unknown TypeFor value: ${typeFor}`);
    }
  }
  return getStringType(language);
}

function getEnumType(enums, language, type) {
  const enumPfx = 'ENUM:';
  if (!type.startsWith(enumPfx)) {
    throw new Error(`Missing '${enumPfx}' enum prefix.`);
  }
  const name = type.substring(enumPfx.length);
  const enumJson = enums[name];
  if (typeof enumJson === 'undefined') {
    throw new Error(`Unknown enum name: ${name}`);
  }
  return getLangEnumName(enumJson, language);
}

function getClassType(interfaces, language, type, classPfx = 'CLASS:') {
  if (!type.startsWith(classPfx)) {
    throw new Error(`Missing '${classPfx}' class prefix.`);
  }
  const name = type.substring(classPfx.length);
  const interfaceJson = interfaces[name];
  if (typeof interfaceJson === 'undefined') {
    throw new Error(`Unknown class name: ${name}`);
  }
  type = getLangClassName(interfaceJson, language);
  if (language === Language.C || language === Language.ObjC) {
    return `${type}*`;
  }
  return type;
}

function getClassRefType(interfaces, language, type, typeFor = TypeFor.Any) {
  type = getClassType(interfaces, language, type, 'CLASS_REF:');
  if (language === Language.CXX) {
    switch (typeFor) {
      case TypeFor.Any:
        return type;
      case TypeFor.PropGetter:
      case TypeFor.PropSetter:
        return `const ${type}&`;
      default:
        throw new Error(`Unknown TypeFor value: ${typeFor}`);
    }
  } else if (language === Language.C) {
    switch (typeFor) {
      case TypeFor.Any:
        return type;
      case TypeFor.PropGetter:
      case TypeFor.PropSetter:
        return `const ${type}`;
      default:
        throw new Error(`Unknown TypeFor value: ${typeFor}`);
    }
  }
  return type;
}

function getClassMixType(interfaces, language, type, typeFor = TypeFor.Any) {
  type = getClassType(interfaces, language, type, 'CLASS_MIX:');
  if (language === Language.CXX) {
    switch (typeFor) {
      case TypeFor.Any:
        return type;
      case TypeFor.PropGetter:
        return `const ${type}&`;
      case TypeFor.PropSetter:
        return type;
      default:
        throw new Error(`Unknown TypeFor value: ${typeFor}`);
    }
  } else if (language === Language.C) {
    switch (typeFor) {
      case TypeFor.Any:
        return type;
      case TypeFor.PropGetter:
        return `const ${type}`;
      case TypeFor.PropSetter:
        return type;
      default:
        throw new Error(`Unknown TypeFor value: ${typeFor}`);
    }
  }
  return type;
}

function getClassPtrType(interfaces, language, type, classPfx = 'CLASS_PTR:') {
  type = getClassType(interfaces, language, type, classPfx);
  if (language === Language.CXX) {
    return `std::shared_ptr<${type}>`;
  } else if (language === Language.C) {
    return `${type.substring(0, type.length - '_t*'.length)}_ref_t*`;
  } else if (language === Language.Swift) {
    return `${type}?`;
  }
  return type;
}

function getClassPtrRefType(interfaces, language, type, typeFor = TypeFor.Any) {
  type = getClassPtrType(interfaces, language, type, 'CLASS_PTR_REF:');
  if (language === Language.CXX) {
    switch (typeFor) {
      case TypeFor.Any:
        return type;
      case TypeFor.PropGetter:
      case TypeFor.PropSetter:
        return `const ${type}&`;
      default:
        throw new Error(`Unknown TypeFor value: ${typeFor}`);
    }
  } else if (language === Language.C) {
    switch (typeFor) {
      case TypeFor.Any:
        return type;
      case TypeFor.PropGetter:
      case TypeFor.PropSetter:
        return `const ${type}`;
      default:
        throw new Error(`Unknown TypeFor value: ${typeFor}`);
    }
  }
  return type;
}

function getClassPtrMixType(interfaces, language, type, typeFor = TypeFor.Any) {
  type = getClassPtrType(interfaces, language, type, 'CLASS_PTR_MIX:');
  if (language === Language.CXX) {
    switch (typeFor) {
      case TypeFor.Any:
        return type;
      case TypeFor.PropGetter:
        return `const ${type}&`;
      case TypeFor.PropSetter:
        return type;
      default:
        throw new Error(`Unknown TypeFor value: ${typeFor}`);
    }
  } else if (language === Language.C) {
    switch (typeFor) {
      case TypeFor.Any:
        return type;
      case TypeFor.PropGetter:
        return `const ${type}`;
      case TypeFor.PropSetter:
        return type;
      default:
        throw new Error(`Unknown TypeFor value: ${typeFor}`);
    }
  }
  return type;
}

function wrapInArrayType(language, type, isBoolean = false, isEnum = false, isClass = false, typeFor = TypeFor.Any) {
  switch (language) {
    case Language.CXX:
      return `std::vector<${type}>`;
    case Language.C:
      switch (typeFor) {
        case TypeFor.Any:
        case TypeFor.PropGetter:
          if (isClass) {
            return `${type}**`;
          }
          return `${type}*`;
        case TypeFor.PropSetter:
          if (type === 'const char*') {
            return `${type} const*`;
          } else if (isClass) {
            return `${type}* const*`;
          }
          return `const ${type}*`;
        default:
          throw new Error(`Unknown TypeFor value: ${typeFor}`);
      }
    case Language.ObjC:
      if (type === 'float' || type === 'double' || type.startsWith('int') || type.startsWith('uint') || type === 'BOOL' || isEnum) {
        return `NSArray<NSNumber*>*`;
      } else if (isClass) {
        return `NSArray<${type}*>*`;
      }
      return `NSArray<${type}>*`;
    case Language.Swift:
      return `[${type}]`;
    case Language.JS:
      return `${type}[]`;
    default:
      throw new Error(`Unknown language: ${language}`);
  }
}

function wrapInArrayRefType(language, type, isBoolean = false, isEnum = false, isClass = false, typeFor = TypeFor.Any) {
  switch (language) {
    case Language.CXX:
      switch (typeFor) {
        case TypeFor.Any:
          break;
        case TypeFor.PropGetter:
        case TypeFor.PropSetter:
          return `const std::vector<${type}>&`;
        default:
          throw new Error(`Unknown TypeFor value: ${typeFor}`);
      }
    case Language.C:
      switch (typeFor) {
        case TypeFor.Any:
          break;
        case TypeFor.PropGetter:
          if (isBoolean || type === 'const char*') {
            break;
          } else if (isClass) {
            return `const ${type}**`;
          }
          return `const ${type}*`;
        case TypeFor.PropSetter:
          if (isClass) {
            return `const ${type}* const*`;
          }
          break;
        default:
          throw new Error(`Unknown TypeFor value: ${typeFor}`);
      }
    case Language.ObjC:
    case Language.Swift:
    case Language.JS:
      break;
    default:
      throw new Error(`Unknown language: ${language}`);
  }
  return wrapInArrayType(language, type, isBoolean, isEnum, isClass, typeFor);
}

function wrapInArrayMixType(language, type, isBoolean = false, isEnum = false, isClass = false, typeFor = TypeFor.Any) {
  switch (language) {
    case Language.CXX:
      switch (typeFor) {
        case TypeFor.Any:
          break;
        case TypeFor.PropGetter:
          return `const std::vector<${type}>&`;
        case TypeFor.PropSetter:
          break;
        default:
          throw new Error(`Unknown TypeFor value: ${typeFor}`);
      }
    case Language.C:
      switch (typeFor) {
        case TypeFor.Any:
          break;
        case TypeFor.PropGetter:
          if (isBoolean || type === 'const char*') {
            break;
          } else if (isClass) {
            return `const ${type}**`;
          }
          return `const ${type}*`;
        case TypeFor.PropSetter:
          break;
        default:
          throw new Error(`Unknown TypeFor value: ${typeFor}`);
      }
    case Language.ObjC:
    case Language.Swift:
    case Language.JS:
      break;
    default:
      throw new Error(`Unknown language: ${language}`);
  }
  return wrapInArrayType(language, type, isBoolean, isEnum, isClass, typeFor);
}

function getArrayType(enums, interfaces, language, type, typeFor = TypeFor.Any, wrapInArrayTypeFunc = wrapInArrayType, arrayPfx = 'ARRAY:') {
  if (!type.startsWith(arrayPfx)) {
    throw new Error(`Missing '${arrayPfx}' array prefix.`);
  }
  const name = type.substring(arrayPfx.length);
  if (name === 'float') {
    return wrapInArrayTypeFunc(language, getFloatType(language), false, false, false, typeFor);
  } else if (name === 'double') {
    return wrapInArrayTypeFunc(language, getDoubleType(language), false, false, false, typeFor);
  } else if (name.startsWith('int')) {
    return wrapInArrayTypeFunc(language, getIntType(true, name.substring(3), language), false, false, false, typeFor);
  } else if (name.startsWith('uint')) {
    return wrapInArrayTypeFunc(language, getIntType(false, name.substring(4), language), false, false, false, typeFor);
  } else if (name === 'boolean') {
    return wrapInArrayTypeFunc(language, getBooleanType(language), true, false, false, typeFor);
  } else if (name === 'string') {
    return wrapInArrayTypeFunc(language, getStringType(language), false, false, false, typeFor);
  }
  const enumJson = enums[name];
  if (typeof enumJson !== 'undefined') {
    return wrapInArrayTypeFunc(language, getLangEnumName(enumJson, language), false, true, false, typeFor);
  }
  const interfaceJson = interfaces[name];
  if (typeof interfaceJson !== 'undefined') {
    return wrapInArrayTypeFunc(language, getLangClassName(interfaceJson, language), false, false, true, typeFor);
  }
  throw new Error(`Unknown enum or class name: ${name}`);
}

function getArrayRefType(enums, interfaces, language, type, typeFor = TypeFor.Any) {
  return getArrayType(enums, interfaces, language, type, typeFor, wrapInArrayRefType, 'ARRAY_REF:');
}

function getArrayMixType(enums, interfaces, language, type, typeFor = TypeFor.Any) {
  return getArrayType(enums, interfaces, language, type, typeFor, wrapInArrayMixType, 'ARRAY_MIX:');
}

function getArrayPtrType(interfaces, language, type, typeFor = TypeFor.Any) {
  const arrayPtrPfx = 'ARRAY_PTR:';
  if (!type.startsWith(arrayPtrPfx)) {
    throw new Error(`Missing '${arrayPtrPfx}' array prefix.`);
  }
  const name = type.substring(arrayPtrPfx.length);
  const interfaceJson = interfaces[name];
  if (typeof interfaceJson === 'undefined') {
    throw new Error(`Unknown class name: ${name}`);
  }
  type = getLangClassName(interfaceJson, language);
  switch (language) {
    case Language.CXX:
      return `std::vector<std::shared_ptr<${type}>>`;
    case Language.C:
      switch (typeFor) {
        case TypeFor.Any:
        case TypeFor.PropGetter:
          return `${type.substring(0, type.length - 2)}_ref_t**`;
        case TypeFor.PropSetter:
          return `${type.substring(0, type.length - 2)}_ref_t* const*`;
        default:
          throw new Error(`Unknown TypeFor value: ${typeFor}`);
      }
    case Language.ObjC:
      return `NSArray<NSObject*>*`;
    case Language.Swift:
      return `[${type}?]`;
    case Language.JS:
      return `${type}[]`;
    default:
      throw new Error(`Unknown language: ${language}`);
  }
}

function getArrayPtrRefType(interfaces, language, type, typeFor = TypeFor.Any) {
  const arrayPtrRefPfx = 'ARRAY_PTR_REF:';
  if (!type.startsWith(arrayPtrRefPfx)) {
    throw new Error(`Missing '${arrayPtrRefPfx}' array prefix.`);
  }
  const name = type.substring(arrayPtrRefPfx.length);
  const interfaceJson = interfaces[name];
  if (typeof interfaceJson === 'undefined') {
    throw new Error(`Unknown class name: ${name}`);
  }
  type = getLangClassName(interfaceJson, language);
  switch (language) {
    case Language.CXX:
      switch (typeFor) {
        case TypeFor.Any:
          return `std::vector<std::shared_ptr<${type}>>`;
        case TypeFor.PropGetter:
        case TypeFor.PropSetter:
          return `const std::vector<std::shared_ptr<${type}>>&`;
        default:
          throw new Error(`Unknown TypeFor value: ${typeFor}`);
      }
    case Language.C:
      switch (typeFor) {
        case TypeFor.Any:
          return `${type.substring(0, type.length - 2)}_ref_t**`;
        case TypeFor.PropGetter:
          return `const ${type.substring(0, type.length - 2)}_ref_t**`;
        case TypeFor.PropSetter:
          return `const ${type.substring(0, type.length - 2)}_ref_t* const*`;
        default:
          throw new Error(`Unknown TypeFor value: ${typeFor}`);
      }
    case Language.ObjC:
      return `NSArray<NSObject*>*`;
    case Language.Swift:
      return `[${type}?]`;
    case Language.JS:
      return `${type}[]`;
    default:
      throw new Error(`Unknown language: ${language}`);
  }
}

function getArrayPtrMixType(interfaces, language, type, typeFor = TypeFor.Any) {
  const arrayPtrMixPfx = 'ARRAY_PTR_MIX:';
  if (!type.startsWith(arrayPtrMixPfx)) {
    throw new Error(`Missing '${arrayPtrMixPfx}' array prefix.`);
  }
  const name = type.substring(arrayPtrMixPfx.length);
  const interfaceJson = interfaces[name];
  if (typeof interfaceJson === 'undefined') {
    throw new Error(`Unknown class name: ${name}`);
  }
  type = getLangClassName(interfaceJson, language);
  switch (language) {
    case Language.CXX:
      switch (typeFor) {
        case TypeFor.Any:
          return `std::vector<std::shared_ptr<${type}>>`;
        case TypeFor.PropGetter:
          return `const std::vector<std::shared_ptr<${type}>>&`;
        case TypeFor.PropSetter:
          return `std::vector<std::shared_ptr<${type}>>`;
        default:
          throw new Error(`Unknown TypeFor value: ${typeFor}`);
      }
    case Language.C:
      switch (typeFor) {
        case TypeFor.Any:
          return `${type.substring(0, type.length - 2)}_ref_t**`;
        case TypeFor.PropGetter:
          return `const ${type.substring(0, type.length - 2)}_ref_t**`;
        case TypeFor.PropSetter:
          return `${type.substring(0, type.length - 2)}_ref_t* const*`;
        default:
          throw new Error(`Unknown TypeFor value: ${typeFor}`);
      }
    case Language.ObjC:
      return `NSArray<NSObject*>*`;
    case Language.Swift:
      return `[${type}?]`;
    case Language.JS:
      return `${type}[]`;
    default:
      throw new Error(`Unknown language: ${language}`);
  }
}

function getPropertyType(enums, interfaces, propertyJson, language) {
  const key = 'type';
  if (typeof propertyJson[key] === 'undefined') {
    throw new Error(`Missing '${key}' key.`);
  }
  if (typeof propertyJson[key] !== 'string') {
    throw new Error(`Invalid '${key}' key type.`);
  }
  if (propertyJson[key].startsWith('int')) {
    return getIntType(true, propertyJson[key].substring(3), language);
  }
  if (propertyJson[key].startsWith('uint')) {
    return getIntType(false, propertyJson[key].substring(4), language);
  }
  if (propertyJson[key].startsWith('ENUM:')) {
    return getEnumType(enums, language, propertyJson[key]);
  }
  if (propertyJson[key].startsWith('CLASS:')) {
    return getClassType(interfaces, language, propertyJson[key]);
  }
  if (propertyJson[key].startsWith('CLASS_REF:')) {
    return getClassRefType(interfaces, language, propertyJson[key]);
  }
  if (propertyJson[key].startsWith('CLASS_MIX:')) {
    return getClassMixType(interfaces, language, propertyJson[key]);
  }
  if (propertyJson[key].startsWith('CLASS_PTR:')) {
    return getClassPtrType(interfaces, language, propertyJson[key]);
  }
  if (propertyJson[key].startsWith('CLASS_PTR_REF:')) {
    return getClassPtrRefType(interfaces, language, propertyJson[key]);
  }
  if (propertyJson[key].startsWith('CLASS_PTR_MIX:')) {
    return getClassPtrMixType(interfaces, language, propertyJson[key]);
  }
  if (propertyJson[key].startsWith('ARRAY:')) {
    return getArrayType(enums, interfaces, language, propertyJson[key]);
  }
  if (propertyJson[key].startsWith('ARRAY_REF:')) {
    return getArrayRefType(enums, interfaces, language, propertyJson[key]);
  }
  if (propertyJson[key].startsWith('ARRAY_MIX:')) {
    return getArrayMixType(enums, interfaces, language, propertyJson[key]);
  }
  if (propertyJson[key].startsWith('ARRAY_PTR:')) {
    return getArrayPtrType(interfaces, language, propertyJson[key]);
  }
  if (propertyJson[key].startsWith('ARRAY_PTR_REF:')) {
    return getArrayPtrRefType(interfaces, language, propertyJson[key]);
  }
  if (propertyJson[key].startsWith('ARRAY_PTR_MIX:')) {
    return getArrayPtrMixType(interfaces, language, propertyJson[key]);
  }
  switch (propertyJson[key]) {
    case 'float':
      return getFloatType(language);
    case 'double':
      return getDoubleType(language);
    case 'boolean':
      return getBooleanType(language);
    case 'string':
      return getStringType(language);
    case 'string_ref':
      return getStringRefType(language);
    case 'string_mix':
      return getStringMixType(language);
    default:
      throw new Error(`Unknown type: ${propertyJson[key]}`);
  }
}

function getPropertyTypeForGetter(enums, interfaces, propertyJson, language) {
  const key = 'type';
  if (typeof propertyJson[key] === 'undefined') {
    throw new Error(`Missing '${key}' key.`);
  }
  if (typeof propertyJson[key] !== 'string') {
    throw new Error(`Invalid '${key}' key type.`);
  }
  if (propertyJson[key].startsWith('int')) {
    return getIntType(true, propertyJson[key].substring(3), language);
  }
  if (propertyJson[key].startsWith('uint')) {
    return getIntType(false, propertyJson[key].substring(4), language);
  }
  if (propertyJson[key].startsWith('ENUM:')) {
    return getEnumType(enums, language, propertyJson[key]);
  }
  if (propertyJson[key].startsWith('CLASS:')) {
    return getClassType(interfaces, language, propertyJson[key]);
  }
  if (propertyJson[key].startsWith('CLASS_REF:')) {
    return getClassRefType(interfaces, language, propertyJson[key], TypeFor.PropGetter);
  }
  if (propertyJson[key].startsWith('CLASS_MIX:')) {
    return getClassMixType(interfaces, language, propertyJson[key], TypeFor.PropGetter);
  }
  if (propertyJson[key].startsWith('CLASS_PTR:')) {
    return getClassPtrType(interfaces, language, propertyJson[key]);
  }
  if (propertyJson[key].startsWith('CLASS_PTR_REF:')) {
    return getClassPtrRefType(interfaces, language, propertyJson[key], TypeFor.PropGetter);
  }
  if (propertyJson[key].startsWith('CLASS_PTR_MIX:')) {
    return getClassPtrMixType(interfaces, language, propertyJson[key], TypeFor.PropGetter);
  }
  if (propertyJson[key].startsWith('ARRAY:')) {
    return getArrayType(enums, interfaces, language, propertyJson[key], TypeFor.PropGetter);
  }
  if (propertyJson[key].startsWith('ARRAY_REF:')) {
    return getArrayRefType(enums, interfaces, language, propertyJson[key], TypeFor.PropGetter);
  }
  if (propertyJson[key].startsWith('ARRAY_MIX:')) {
    return getArrayMixType(enums, interfaces, language, propertyJson[key], TypeFor.PropGetter);
  }
  if (propertyJson[key].startsWith('ARRAY_PTR:')) {
    return getArrayPtrType(interfaces, language, propertyJson[key], TypeFor.PropGetter);
  }
  if (propertyJson[key].startsWith('ARRAY_PTR_REF:')) {
    return getArrayPtrRefType(interfaces, language, propertyJson[key], TypeFor.PropGetter);
  }
  if (propertyJson[key].startsWith('ARRAY_PTR_MIX:')) {
    return getArrayPtrMixType(interfaces, language, propertyJson[key], TypeFor.PropGetter);
  }
  switch (propertyJson[key]) {
    case 'float':
      return getFloatType(language);
    case 'double':
      return getDoubleType(language);
    case 'boolean':
      return getBooleanType(language);
    case 'string':
      return getStringType(language);
    case 'string_ref':
      return getStringRefType(language, TypeFor.PropGetter);
    case 'string_mix':
      return getStringMixType(language, TypeFor.PropGetter);
    default:
      throw new Error(`Unknown type: ${propertyJson[key]}`);
  }
}

function getPropertyTypeForSetter(enums, interfaces, propertyJson, language) {
  const key = 'type';
  if (typeof propertyJson[key] === 'undefined') {
    throw new Error(`Missing '${key}' key.`);
  }
  if (typeof propertyJson[key] !== 'string') {
    throw new Error(`Invalid '${key}' key type.`);
  }
  if (propertyJson[key].startsWith('int')) {
    return getIntType(true, propertyJson[key].substring(3), language);
  }
  if (propertyJson[key].startsWith('uint')) {
    return getIntType(false, propertyJson[key].substring(4), language);
  }
  if (propertyJson[key].startsWith('ENUM:')) {
    return getEnumType(enums, language, propertyJson[key]);
  }
  if (propertyJson[key].startsWith('CLASS:')) {
    return getClassType(interfaces, language, propertyJson[key]);
  }
  if (propertyJson[key].startsWith('CLASS_REF:')) {
    return getClassRefType(interfaces, language, propertyJson[key], TypeFor.PropSetter);
  }
  if (propertyJson[key].startsWith('CLASS_MIX:')) {
    return getClassMixType(interfaces, language, propertyJson[key], TypeFor.PropSetter);
  }
  if (propertyJson[key].startsWith('CLASS_PTR:')) {
    return getClassPtrType(interfaces, language, propertyJson[key]);
  }
  if (propertyJson[key].startsWith('CLASS_PTR_REF:')) {
    return getClassPtrRefType(interfaces, language, propertyJson[key], TypeFor.PropSetter);
  }
  if (propertyJson[key].startsWith('CLASS_PTR_MIX:')) {
    return getClassPtrMixType(interfaces, language, propertyJson[key], TypeFor.PropSetter);
  }
  if (propertyJson[key].startsWith('ARRAY:')) {
    return getArrayType(enums, interfaces, language, propertyJson[key], TypeFor.PropSetter);
  }
  if (propertyJson[key].startsWith('ARRAY_REF:')) {
    return getArrayRefType(enums, interfaces, language, propertyJson[key], TypeFor.PropSetter);
  }
  if (propertyJson[key].startsWith('ARRAY_MIX:')) {
    return getArrayMixType(enums, interfaces, language, propertyJson[key], TypeFor.PropSetter);
  }
  if (propertyJson[key].startsWith('ARRAY_PTR:')) {
    return getArrayPtrType(interfaces, language, propertyJson[key], TypeFor.PropSetter);
  }
  if (propertyJson[key].startsWith('ARRAY_PTR_REF:')) {
    return getArrayPtrRefType(interfaces, language, propertyJson[key], TypeFor.PropSetter);
  }
  if (propertyJson[key].startsWith('ARRAY_PTR_MIX:')) {
    return getArrayPtrMixType(interfaces, language, propertyJson[key], TypeFor.PropSetter);
  }
  switch (propertyJson[key]) {
    case 'float':
      return getFloatType(language);
    case 'double':
      return getDoubleType(language);
    case 'boolean':
      return getBooleanType(language);
    case 'string':
      return getStringType(language);
    case 'string_ref':
      return getStringRefType(language, TypeFor.PropSetter);
    case 'string_mix':
      return getStringMixType(language, TypeFor.PropSetter);
    default:
      throw new Error(`Unknown type: ${propertyJson[key]}`);
  }
}

function getPropertyGetterMode(propertyJson) {
  return propertyJson.getterMode;
}

function getPropertySetterMode(propertyJson) {
  return propertyJson.setterMode;
}

function getPropertyGetterCustom(propertyJson) {
  return propertyJson.getterCustom;
}

function getPropertySetterCustom(propertyJson) {
  return propertyJson.setterCustom;
}

function getPropertyGetterVisibility(propertyJson) {
  return propertyJson.getterVisibility;
}

function getPropertySetterVisibility(propertyJson) {
  return propertyJson.setterVisibility;
}

function getPropertyHasMemberVar(propertyJson) {
  return propertyJson.hasMemberVar;
}

function getPropertyDefaultValue(propertyJson) {
  return propertyJson.defaultValue;
}

function getPropertyStatic(propertyJson) {
  return propertyJson.static;
}

function isIntType(type) {
  return type.startsWith('int') || type.startsWith('uint');
}

function isEnumType(type) {
  return type.startsWith('ENUM:');
}

function isClassType(type) {
  return type.startsWith('CLASS:');
}

function isClassRefType(type) {
  return type.startsWith('CLASS_REF:');
}

function isClassMixType(type) {
  return type.startsWith('CLASS_MIX:');
}

function isClassPtrType(type) {
  return type.startsWith('CLASS_PTR:');
}

function isClassPtrRefType(type) {
  return type.startsWith('CLASS_PTR_REF:');
}

function isClassPtrMixType(type) {
  return type.startsWith('CLASS_PTR_MIX:');
}

function isClassOfAnyType(type) {
  return isClassType(type) || isClassRefType(type) || isClassMixType(type) ||
    isClassPtrType(type) || isClassPtrRefType(type) || isClassPtrMixType(type);
}

function isArrayType(type) {
  return type.startsWith('ARRAY:');
}

function isArrayRefType(type) {
  return type.startsWith('ARRAY_REF:');
}

function isArrayMixType(type) {
  return type.startsWith('ARRAY_MIX:');
}

function isArrayPtrType(type) {
  return type.startsWith('ARRAY_PTR:');
}

function isArrayPtrRefType(type) {
  return type.startsWith('ARRAY_PTR_REF:');
}

function isArrayPtrMixType(type) {
  return type.startsWith('ARRAY_PTR_MIX:');
}

function isArrayOfAnyType(type) {
  return isArrayType(type) || isArrayRefType(type) || isArrayMixType(type) ||
    isArrayPtrType(type) || isArrayPtrRefType(type) || isArrayPtrMixType(type);
}

function isPrimitiveType(type) {
  if (isIntType(type)) {
    return true;
  }
  if (isEnumType(type)) {
    return true;
  }
  if (isClassOfAnyType(type)) {
    return false;
  }
  if (isArrayOfAnyType(type)) {
    return false;
  }
  switch (type) {
    case 'float':
    case 'double':
    case 'boolean':
      return true;
    case 'string':
    case 'string_ref':
    case 'string_mix':
      return false;
    default:
      return false;
  }
}

function getTypeImplicitDefaultValue(type) {
  if (isIntType(type)) {
    return '0';
  }
  if (isEnumType(type)) {
    return '';
  }
  if (isClassOfAnyType(type)) {
    return '';
  }
  if (isArrayOfAnyType(type)) {
    return '';
  }
  switch (type) {
    case 'float':
      return '0.0f';
    case 'double':
      return '0.0';
    case 'boolean':
      return 'false';
    case 'string':
    case 'string_ref':
    case 'string_mix':
      return '';
    default:
      return '';
  }
}

function getTypeMembVarCopyOp(type, membVar) {
  if (isIntType(type)) {
    return membVar;
  }
  if (isEnumType(type)) {
    return membVar;
  }
  if (isClassType(type) || isClassRefType(type) || isClassMixType(type)) {
    return membVar;
  }
  if (isClassPtrType(type) || isClassPtrRefType(type) || isClassPtrMixType(type)) {
    return `std::make_shared<decltype(${membVar})::element_type>(*${membVar})`;
  }
  if (isArrayType(type) || isArrayRefType(type) || isArrayMixType(type)) {
    return membVar;
  }
  if (isArrayPtrType(type) || isArrayPtrRefType(type) || isArrayPtrMixType(type)) {
    return `deepCopyArrayPtr<decltype(${membVar})::value_type::element_type>(${membVar})`;
  }
  switch (type) {
    case 'float':
    case 'double':
    case 'boolean':
    case 'string':
    case 'string_ref':
    case 'string_mix':
      return membVar;
    default:
      return membVar;
  }
}

function getTypeMembVarMoveOp(type, membVar) {
  if (isIntType(type)) {
    return membVar;
  }
  if (isEnumType(type)) {
    return membVar;
  }
  if (isClassOfAnyType(type)) {
    return `std::move(${membVar})`;
  }
  if (isArrayOfAnyType(type)) {
    return `std::move(${membVar})`;
  }
  switch (type) {
    case 'float':
    case 'double':
    case 'boolean':
      return membVar;
    case 'string':
    case 'string_ref':
    case 'string_mix':
      return `std::move(${membVar})`;
    default:
      return `std::move(${membVar})`;
  }
}

function getTypePropEqOp(type, clsParam, prpParam) {
  if (isIntType(type)) {
    return `${prpParam} == ${clsParam}.${prpParam}`;
  }
  if (isEnumType(type)) {
    return `${prpParam} == ${clsParam}.${prpParam}`;
  }
  if (isClassType(type) || isClassRefType(type) || isClassMixType(type)) {
    return `${prpParam} == ${clsParam}.${prpParam}`;
  }
  if (isClassPtrType(type) || isClassPtrRefType(type) || isClassPtrMixType(type)) {
    return `static_cast<bool>(${prpParam}) == static_cast<bool>(${clsParam}.${prpParam}) && (!${prpParam} || *${prpParam} == *(${clsParam}.${prpParam}))`;
  }
  if (isArrayType(type) || isArrayRefType(type) || isArrayMixType(type)) {
    return `${prpParam} == ${clsParam}.${prpParam}`;
  }
  if (isArrayPtrType(type) || isArrayPtrRefType(type) || isArrayPtrMixType(type)) {
    return `equalsArrayPtr<decltype(${prpParam})::value_type::element_type>(${prpParam}, ${clsParam}.${prpParam})`;
  }
  switch (type) {
    case 'float':
    case 'double':
    case 'boolean':
    case 'string':
    case 'string_ref':
    case 'string_mix':
      return `${prpParam} == ${clsParam}.${prpParam}`;
    default:
      return `${prpParam} == ${clsParam}.${prpParam}`;
  }
}

function getTypePropHasher(type, param) {
  if (isIntType(type)) {
    return `hash<${type}_t> {}(${param})`;
  }
  if (isEnumType(type)) {
    return `hash<decltype(${param})> {}(${param})`;
  }
  if (isClassType(type) || isClassRefType(type) || isClassMixType(type)) {
    return `hash<decltype(${param})> {}(${param})`;
  }
  if (isClassPtrType(type) || isClassPtrRefType(type) || isClassPtrMixType(type)) {
    return `${param} ? hash<decltype(${param})::element_type> {}(*${param}) : 0`;
  }
  if (isArrayType(type) || isArrayRefType(type) || isArrayMixType(type)) {
    return `hashArray<decltype(${param})::value_type>(${param})`;
  }
  if (isArrayPtrType(type) || isArrayPtrRefType(type) || isArrayPtrMixType(type)) {
    return `hashArrayPtr<decltype(${param})::value_type::element_type>(${param})`;
  }
  switch (type) {
    case 'float':
    case 'double':
      return `hash<${type}> {}(${param})`;
    case 'boolean':
      return `hash<bool> {}(${param})`;
    case 'string':
    case 'string_ref':
    case 'string_mix':
      return `hash<string> {}(${param})`;
    default:
      return `hash<${type}> {}(${param})`;
  }
}

let defaultPropGetterNameLangToTransformationMap = {};
defaultPropGetterNameLangToTransformationMap[Language.CXX] = 'get%';
defaultPropGetterNameLangToTransformationMap[Language.C] = 'get_%';
defaultPropGetterNameLangToTransformationMap[Language.ObjC] = '%';
defaultPropGetterNameLangToTransformationMap[Language.Swift] = 'get%'; // don't care
defaultPropGetterNameLangToTransformationMap[Language.JS] = 'get%'; // don't care

function getPropertyGetterName(propertyJson, language, nameLangToTransformationMap = defaultPropGetterNameLangToTransformationMap) {
  return getLangTransformedName('getterName', propertyJson, language, nameLangToTransformationMap);
}

let defaultPropSetterNameLangToTransformationMap = {};
defaultPropSetterNameLangToTransformationMap[Language.CXX] = 'set%';
defaultPropSetterNameLangToTransformationMap[Language.C] = 'set_%';
defaultPropSetterNameLangToTransformationMap[Language.ObjC] = 'set%';
defaultPropSetterNameLangToTransformationMap[Language.Swift] = 'set%'; // don't care
defaultPropSetterNameLangToTransformationMap[Language.JS] = 'set%'; // don't care

function getPropertySetterName(propertyJson, language, nameLangToTransformationMap = defaultPropSetterNameLangToTransformationMap) {
  return getLangTransformedName('setterName', propertyJson, language, nameLangToTransformationMap);
}

let defaultPropSetterArgNameLangToTransformationMap = {};
defaultPropSetterArgNameLangToTransformationMap[Language.CXX] = '%';
defaultPropSetterArgNameLangToTransformationMap[Language.C] = '%';
defaultPropSetterArgNameLangToTransformationMap[Language.ObjC] = '%';
defaultPropSetterArgNameLangToTransformationMap[Language.Swift] = '%'; // don't care
defaultPropSetterArgNameLangToTransformationMap[Language.JS] = '%'; // don't care

function getPropertySetterArgName(propertyJson, language, nameLangToTransformationMap = defaultPropSetterArgNameLangToTransformationMap) {
  return getLangTransformedName('setterArgName', propertyJson, language, nameLangToTransformationMap);
}

function getProperties(interfaceJson) {
  return interfaceJson.properties;
}

// name must be in camel snake case
function convertNameStyle(name, nameStyle) {
  if (name.length === 0) {
    throw new Error('Name is empty.');
  }
  if (name[0] >= '0' && name[0] <= '9') {
    throw new Error('Name starts with a number.');
  }
  const segments = name.split('_');
  for (const segment of segments) {
    if (segment.length === 0) {
      throw new Error('Name contains empty segment(s).');
    }
    if (!/^[a-zA-Z0-9]*$/.test(segment)) {
      throw new Error('Name contains invalid character(s).');
    }
  }
  let convertedName = '';
  switch (nameStyle) {
    case NameStyle.lower_case:
      for (const segment of segments) {
        if (convertedName.length === 0) {
          convertedName = segment.toLowerCase();
        } else {
          convertedName += '_' + segment.toLowerCase();
        }
      }
      break;
    case NameStyle.UPPER_CASE:
      for (const segment of segments) {
        if (convertedName.length === 0) {
          convertedName = segment.toUpperCase();
        } else {
          convertedName += '_' + segment.toUpperCase();
        }
      }
      break;
    case NameStyle.camelBack:
      for (const segment of segments) {
        if (convertedName.length === 0) {
          convertedName = segment.toLowerCase();
        } else {
          let convertedSegment = segment.toLowerCase();
          convertedSegment = convertedSegment[0].toUpperCase() + convertedSegment.substring(1);
          convertedName += convertedSegment;
        }
      }
      break;
    case NameStyle.CamelCase:
      for (const segment of segments) {
        let convertedSegment = segment.toLowerCase();
        convertedSegment = convertedSegment[0].toUpperCase() + convertedSegment.substring(1);
        convertedName += convertedSegment;
      }
      break;
    case NameStyle.camel_Snake_Back:
      for (const segment of segments) {
        if (convertedName.length === 0) {
          convertedName = segment.toLowerCase();
        } else {
          let convertedSegment = segment.toLowerCase();
          convertedSegment = convertedSegment[0].toUpperCase() + convertedSegment.substring(1);
          convertedName += '_' + convertedSegment;
        }
      }
      break;
    case NameStyle.Camel_Snake_Case:
      for (const segment of segments) {
        let convertedSegment = segment.toLowerCase();
        convertedSegment = convertedSegment[0].toUpperCase() + convertedSegment.substring(1);
        if (convertedName.length === 0) {
          convertedName += convertedSegment;
        } else {
          convertedName += '_' + convertedSegment;
        }
      }
      break;
    case NameStyle.Leading_upper_snake_case:
      for (const segment of segments) {
        if (convertedName.length === 0) {
          convertedName = segment.toLowerCase();
        } else {
          convertedName += '_' + segment.toLowerCase();
        }
      }
      convertedName = convertedName[0].toUpperCase() + convertedName.substring(1);
      break;
    default:
      throw new Error(`Unknown name style: ${nameStyle}`);
  }
  return convertedName;
}

let defaultEnumNameLangToStyleMap = {};
defaultEnumNameLangToStyleMap[Language.CXX] = NameStyle.CamelCase;
defaultEnumNameLangToStyleMap[Language.C] = NameStyle.lower_case;
defaultEnumNameLangToStyleMap[Language.ObjC] = NameStyle.CamelCase;
defaultEnumNameLangToStyleMap[Language.Swift] = NameStyle.CamelCase;
defaultEnumNameLangToStyleMap[Language.JS] = NameStyle.CamelCase;

let defaultEnumConstantNameLangToStyleMap = {};
defaultEnumConstantNameLangToStyleMap[Language.CXX] = NameStyle.CamelCase;
defaultEnumConstantNameLangToStyleMap[Language.C] = NameStyle.UPPER_CASE;
defaultEnumConstantNameLangToStyleMap[Language.ObjC] = NameStyle.CamelCase;
defaultEnumConstantNameLangToStyleMap[Language.Swift] = NameStyle.camelBack; // don't care
defaultEnumConstantNameLangToStyleMap[Language.JS] = NameStyle.UPPER_CASE;

let defaultClassNameLangToStyleMap = {};
defaultClassNameLangToStyleMap[Language.CXX] = NameStyle.CamelCase;
defaultClassNameLangToStyleMap[Language.C] = NameStyle.lower_case;
defaultClassNameLangToStyleMap[Language.ObjC] = NameStyle.CamelCase;
defaultClassNameLangToStyleMap[Language.Swift] = NameStyle.CamelCase;
defaultClassNameLangToStyleMap[Language.JS] = NameStyle.CamelCase;

function fillName(key, json, nameLangToStyleMap) {
  const name = getName(key, json);
  if (typeof json[`${key}.gen`] === 'undefined') {
    json[`${key}.gen`] = false;
  }
  for (const [styleKey, styleValue] of Object.entries(NameStylePretty)) {
    const nameKey = `${key}.style.${styleKey}`;
    const nameKeyGen = `${nameKey}.gen`;
    if (typeof json[nameKey] === 'undefined') {
      json[nameKey] = convertNameStyle(name, styleValue);
      json[nameKeyGen] = true;
    } else if (typeof json[nameKey] === 'string') {
      const styledName = json[nameKey];
      if (styledName.length === 0) {
        throw new Error(`Empty '${nameKey}' key.`);
      }
      if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(styledName)) {
        throw new Error(`Invalid '${nameKey}' key value.`);
      }
      json[nameKeyGen] = false;
    } else {
      throw new Error(`Invalid '${nameKey}' key type.`);
    }
  }
  for (const [langKey, langValue] of Object.entries(LanguagePretty)) {
    const nameKey = `${key}.lang.${langKey}`;
    const nameKeyGen = `${nameKey}.gen`;
    if (typeof json[nameKey] === 'undefined') {
      json[nameKey] = convertNameStyle(name, nameLangToStyleMap[langValue]);
      json[nameKeyGen] = true;
    } else if (typeof json[nameKey] === 'string') {
      const styledName = json[nameKey];
      if (styledName.length === 0) {
        throw new Error(`Empty '${nameKey}' key.`);
      }
      if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(styledName)) {
        throw new Error(`Invalid '${nameKey}' key value.`);
      }
      json[nameKeyGen] = false;
    } else {
      throw new Error(`Invalid '${nameKey}' key type.`);
    }
  }
}

function fillEnumName(enumJson, nameLangToStyleMap = defaultEnumNameLangToStyleMap) {
  fillName('name', enumJson, nameLangToStyleMap);
}

function fillEnumType(enumJson) {
  const nameKey = 'type';
  const nameKeyGen = `${nameKey}.gen`;
  if (typeof enumJson[nameKey] === 'undefined') {
    enumJson[nameKey] = 'default';
    enumJson[nameKeyGen] = true;
  } else if (typeof enumJson[nameKey] === 'string') {
    const type = enumJson[nameKey];
    if (type !== 'default' && type !== 'flag') {
      throw new Error(`Invalid '${nameKey}' key value.`);
    }
    enumJson[nameKeyGen] = false;
  } else {
    throw new Error(`Invalid '${nameKey}' key type.`);
  }
}

function fillEnumConstantName(constantJson, nameLangToStyleMap = defaultEnumConstantNameLangToStyleMap) {
  fillName('name', constantJson, nameLangToStyleMap);
}

function fillEnumConstants(enumJson, nameLangToStyleMap = defaultEnumConstantNameLangToStyleMap) {
  const nameKey = 'constants';
  const nameKeyGen = `${nameKey}.gen`;
  if (typeof enumJson[nameKey] === 'undefined') {
    enumJson[nameKey] = [];
    enumJson[nameKeyGen] = true;
  } else if (Array.isArray(enumJson[nameKey])) {
    const isFlagType = enumJson['type'] === 'flag';
    let nextConstantValue = isFlagType ? 1 : 0;
    let hasUndefinedValues = false;
    let hasDefinedValues = false;
    for (const constantJson of enumJson[nameKey]) {
      fillEnumConstantName(constantJson, nameLangToStyleMap);
      if (typeof constantJson['value'] === 'undefined') {
        if (isFlagType) {
          if (nextConstantValue < 0 || nextConstantValue > 4294967295) {
            throw new Error(`Auto-generated constant value '${nextConstantValue}' is outside of the uint32 domain range.`);
          }
        } else if (nextConstantValue < -2147483648 || nextConstantValue > 2147483647) {
          throw new Error(`Auto-generated constant value '${nextConstantValue}' is outside of the int32 domain range.`);
        }
        hasUndefinedValues = true;
        constantJson['value'] = nextConstantValue;
        constantJson['value.gen'] = true;
        if (isFlagType) {
          nextConstantValue = nextConstantValue * 2;
          if (hasDefinedValues) {
            throw new Error(`Enum '${enumJson['name']}' is missing a value for '${constantJson['name']}' constant. Flag type enums require either all or none of the values to be defined explicitly.`);
          }
        } else {
          nextConstantValue = nextConstantValue + 1;
        }
      } else if (typeof constantJson['value'] === 'number' && Number.isInteger(constantJson['value'])) {
        if (isFlagType) {
          if (constantJson['value'] < 0 || constantJson['value'] > 4294967295) {
            throw new Error(`Explicit constant value '${constantJson['value']}' is outside of the uint32 domain range.`);
          }
        } else if (constantJson['value'] < -2147483648 || constantJson['value'] > 2147483647) {
          throw new Error(`Explicit constant value '${constantJson['value']}' is outside of the int32 domain range.`);
        }
        hasDefinedValues = true;
        constantJson['value.gen'] = false;
        if (isFlagType) {
          nextConstantValue = constantJson['value'] * 2;
          if (hasUndefinedValues) {
            throw new Error(`Enum '${enumJson['name']}' has a value for '${constantJson['name']}' constant. Flag type enums require either all or none of the values to be defined explicitly.`);
          }
        } else {
          nextConstantValue = constantJson['value'] + 1;
        }
      } else {
        throw new Error(`Invalid 'value' key type.`);
      }
    }
    enumJson[nameKeyGen] = false;
  } else {
    throw new Error(`Invalid '${nameKey}' key type.`);
  }
}

function fillClassName(interfaceJson, nameLangToStyleMap = defaultClassNameLangToStyleMap) {
  fillName('name', interfaceJson, nameLangToStyleMap);
}

function fillClassStatic(interfaceJson) {
  if (typeof interfaceJson.static === 'undefined') {
    interfaceJson.static = false;
    interfaceJson['static.gen'] = true;
  } else if (typeof interfaceJson.static !== 'boolean') {
    throw new Error(`Invalid 'static' key type.`);
  } else {
    interfaceJson['static.gen'] = false;
  }
}

function fillClassFinal(interfaceJson) {
  if (typeof interfaceJson.final === 'undefined') {
    const classStatic = getClassStatic(interfaceJson);
    interfaceJson.final = classStatic;
    interfaceJson['final.gen'] = true;
  } else if (typeof interfaceJson.final !== 'boolean') {
    throw new Error(`Invalid 'final' key type.`);
  } else {
    interfaceJson['final.gen'] = false;
  }
}

function fillAliases(interfaceJson, nameLangToStyleMap = defaultClassNameLangToStyleMap) {
  const nameKey = 'aliases';
  const nameKeyGen = `${nameKey}.gen`;
  if (typeof interfaceJson[nameKey] === 'undefined') {
    interfaceJson[nameKey] = [];
    interfaceJson[nameKeyGen] = true;
    return;
  }
  if (!Array.isArray(interfaceJson[nameKey])) {
    throw new Error(`Invalid '${nameKey}' key type.`);
  }
  for (const aliasJson of interfaceJson[nameKey]) {
    fillClassName(aliasJson, nameLangToStyleMap);
  }
  interfaceJson[nameKeyGen] = false;
}

function fillHeaderGuardName(interfaceJson) {
  const nameKey = 'headerGuardName';
  const nameKeyGen = `${nameKey}.gen`;
  if (typeof interfaceJson[nameKey] === 'undefined') {
    interfaceJson[nameKey] = getStyleName('name', interfaceJson, NameStyle.UPPER_CASE);
    interfaceJson[nameKeyGen] = true;
  } else if (typeof interfaceJson[nameKey] === 'string') {
    const headerGuardName = interfaceJson[nameKey];
    if (headerGuardName.length === 0) {
      throw new Error(`Empty '${nameKey}' key.`);
    }
    if (!/^[a-zA-Z0-9_]*$/.test(headerGuardName)) {
      throw new Error(`Invalid '${nameKey}' key value.`);
    }
    interfaceJson[nameKeyGen] = false;
  } else {
    throw new Error(`Invalid '${nameKey}' key type.`);
  }
}

let defaultPropNameLangToStyleMap = {};
defaultPropNameLangToStyleMap[Language.CXX] = NameStyle.camelBack;
defaultPropNameLangToStyleMap[Language.C] = NameStyle.lower_case; // don't care
defaultPropNameLangToStyleMap[Language.ObjC] = NameStyle.camelBack; // don't care
defaultPropNameLangToStyleMap[Language.Swift] = NameStyle.camelBack;
defaultPropNameLangToStyleMap[Language.JS] = NameStyle.camelBack;

let defaultGetterNameLangToStyleMap = {};
defaultGetterNameLangToStyleMap[Language.CXX] = NameStyle.CamelCase;
defaultGetterNameLangToStyleMap[Language.C] = NameStyle.lower_case;
defaultGetterNameLangToStyleMap[Language.ObjC] = NameStyle.camelBack;
defaultGetterNameLangToStyleMap[Language.Swift] = NameStyle.CamelCase; // don't care
defaultGetterNameLangToStyleMap[Language.JS] = NameStyle.CamelCase; // don't care

let defaultSetterNameLangToStyleMap = {};
defaultSetterNameLangToStyleMap[Language.CXX] = NameStyle.CamelCase;
defaultSetterNameLangToStyleMap[Language.C] = NameStyle.lower_case;
defaultSetterNameLangToStyleMap[Language.ObjC] = NameStyle.CamelCase;
defaultSetterNameLangToStyleMap[Language.Swift] = NameStyle.CamelCase; // don't care
defaultSetterNameLangToStyleMap[Language.JS] = NameStyle.CamelCase; // don't care

let defaultSetterArgNameLangToStyleMap = {};
defaultSetterArgNameLangToStyleMap[Language.CXX] = NameStyle.camelBack;
defaultSetterArgNameLangToStyleMap[Language.C] = NameStyle.lower_case;
defaultSetterArgNameLangToStyleMap[Language.ObjC] = NameStyle.camelBack;
defaultSetterArgNameLangToStyleMap[Language.Swift] = NameStyle.camelBack; // don't care
defaultSetterArgNameLangToStyleMap[Language.JS] = NameStyle.camelBack; // don't care

function fillProperty(interfaceJson, propertyJson, nameLangToStyleMap = defaultPropNameLangToStyleMap, getterNameLangToStyleMap = defaultGetterNameLangToStyleMap, setterNameLangToStyleMap = defaultSetterNameLangToStyleMap, setterArgNameLangToStyleMap = defaultSetterArgNameLangToStyleMap) {
  fillName('name', propertyJson, nameLangToStyleMap);

  if (typeof propertyJson.type === 'undefined') {
    throw new Error(`Missing 'type' key.`);
  } else if (typeof propertyJson.type !== 'string') {
    throw new Error(`Invalid 'type' key type.`);
  }
  propertyJson['type.gen'] = false;

  if (typeof propertyJson.static === 'undefined') {
    const classStatic = getClassStatic(interfaceJson);
    propertyJson.static = classStatic;
    propertyJson['static.gen'] = true;
  } else if (typeof propertyJson.static !== 'boolean') {
    throw new Error(`Invalid 'static' key type.`);
  } else {
    propertyJson['static.gen'] = false;
  }

  if (typeof propertyJson.hasGetter === 'undefined') {
    propertyJson.hasGetter = true;
    propertyJson['hasGetter.gen'] = true;
  } else if (typeof propertyJson.hasGetter !== 'boolean') {
    throw new Error(`Invalid 'hasGetter' key type.`);
  } else {
    propertyJson['hasGetter.gen'] = false;
  }

  if (typeof propertyJson.getterConst === 'undefined') {
    const propStatic = getPropertyStatic(propertyJson);
    propertyJson.getterConst = !propStatic;
    propertyJson['getterConst.gen'] = true;
  } else if (typeof propertyJson.getterConst !== 'boolean') {
    throw new Error(`Invalid 'getterConst' key type.`);
  } else {
    propertyJson['getterConst.gen'] = false;
  }

  if (typeof propertyJson.getterNoexcept === 'undefined') {
    if (propertyJson.type === 'string_ref' || propertyJson.type === 'string_mix') {
      propertyJson.getterNoexcept = true;
    } else if (isClassRefType(propertyJson.type) || isClassMixType(propertyJson.type)) {
      propertyJson.getterNoexcept = true;
    } else if (isClassPtrRefType(propertyJson.type) || isClassPtrMixType(propertyJson.type)) {
      propertyJson.getterNoexcept = true;
    } else if (isArrayRefType(propertyJson.type) || isArrayMixType(propertyJson.type)) {
      propertyJson.getterNoexcept = true;
    } else if (isArrayPtrRefType(propertyJson.type) || isArrayPtrMixType(propertyJson.type)) {
      propertyJson.getterNoexcept = true;
    } else {
      propertyJson.getterNoexcept = isPrimitiveType(propertyJson.type);
    }
    propertyJson['getterNoexcept.gen'] = true;
  } else if (typeof propertyJson.getterNoexcept !== 'boolean') {
    throw new Error(`Invalid 'getterNoexcept' key type.`);
  } else {
    propertyJson['getterNoexcept.gen'] = false;
  }

  if (typeof propertyJson.getterName === 'undefined') {
    propertyJson.getterName = propertyJson.name;
    propertyJson['getterName.gen'] = true;
  } else if (typeof propertyJson.getterName !== 'string') {
    throw new Error(`Invalid 'getterName' key type.`);
  } else {
    propertyJson['getterName.gen'] = false;
  }
  fillName('getterName', propertyJson, getterNameLangToStyleMap);

  if (typeof propertyJson.getterMode === 'undefined') {
    propertyJson.getterMode = MethodMode.regular; // TODO: perhaps determine from base class(es)
    propertyJson['getterMode.gen'] = true;
  } else if (typeof propertyJson.getterMode !== 'string') {
    throw new Error(`Invalid 'getterMode' key type.`);
  } else {
    let found = false;
    for (const [key, value] of Object.entries(MethodMode)) {
      if (propertyJson.getterMode === value) {
        found = true;
        break;
      }
    }
    if (!found) {
      throw new Error(`Unknown 'getterMode' value: ${propertyJson.getterMode}`);
    }
    propertyJson['getterMode.gen'] = false;
  }

  if (typeof propertyJson.getterCustom === 'undefined') {
    const getterMode = getPropertyGetterMode(propertyJson);
    propertyJson.getterCustom = getterMode === MethodMode.pureVirtual;
    propertyJson['getterCustom.gen'] = true;
  } else if (typeof propertyJson.getterCustom !== 'boolean') {
    throw new Error(`Invalid 'getterCustom' key type.`);
  } else {
    propertyJson['getterCustom.gen'] = false;
  }

  if (typeof propertyJson.getterVisibility === 'undefined') {
    propertyJson.getterVisibility = Visibility.public; // TODO: perhaps determine from base class(es)
    propertyJson['getterVisibility.gen'] = true;
  } else if (typeof propertyJson.getterVisibility !== 'string') {
    throw new Error(`Invalid 'getterVisibility' key type.`);
  } else {
    let found = false;
    for (const [key, value] of Object.entries(Visibility)) {
      if (propertyJson.getterVisibility === value) {
        found = true;
        break;
      }
    }
    if (!found) {
      throw new Error(`Unknown 'getterVisibility' value: ${propertyJson.getterVisibility}`);
    }
    propertyJson['getterVisibility.gen'] = false;
  }

  if (typeof propertyJson.hasSetter === 'undefined') {
    propertyJson.hasSetter = true;
    propertyJson['hasSetter.gen'] = true;
  } else if (typeof propertyJson.hasSetter !== 'boolean') {
    throw new Error(`Invalid 'hasSetter' key type.`);
  } else {
    propertyJson['hasSetter.gen'] = false;
  }

  if (typeof propertyJson.setterConst === 'undefined') {
    propertyJson.setterConst = false;
    propertyJson['setterConst.gen'] = true;
  } else if (typeof propertyJson.setterConst !== 'boolean') {
    throw new Error(`Invalid 'setterConst' key type.`);
  } else {
    propertyJson['setterConst.gen'] = false;
  }

  if (typeof propertyJson.setterNoexcept === 'undefined') {
    if (propertyJson.type === 'string' || propertyJson.type === 'string_mix') {
      propertyJson.setterNoexcept = true;
    } else if (isClassType(propertyJson.type) || isClassMixType(propertyJson.type)) {
      propertyJson.setterNoexcept = true;
    } else if (isClassPtrType(propertyJson.type) || isClassPtrMixType(propertyJson.type)) {
      propertyJson.setterNoexcept = true;
    } else if (isArrayType(propertyJson.type) || isArrayMixType(propertyJson.type)) {
      propertyJson.setterNoexcept = true;
    } else if (isArrayPtrType(propertyJson.type) || isArrayPtrMixType(propertyJson.type)) {
      propertyJson.setterNoexcept = true;
    } else {
      propertyJson.setterNoexcept = isPrimitiveType(propertyJson.type);
    }
    propertyJson['setterNoexcept.gen'] = true;
  } else if (typeof propertyJson.setterNoexcept !== 'boolean') {
    throw new Error(`Invalid 'setterNoexcept' key type.`);
  } else {
    propertyJson['setterNoexcept.gen'] = false;
  }

  if (typeof propertyJson.setterName === 'undefined') {
    propertyJson.setterName = propertyJson.name;
    propertyJson['setterName.gen'] = true;
  } else if (typeof propertyJson.setterName !== 'string') {
    throw new Error(`Invalid 'setterName' key type.`);
  } else {
    propertyJson['setterName.gen'] = false;
  }
  fillName('setterName', propertyJson, setterNameLangToStyleMap);

  if (typeof propertyJson.setterArgName === 'undefined') {
    propertyJson.setterArgName = propertyJson.name;
    propertyJson['setterArgName.gen'] = true;
  } else if (typeof propertyJson.setterArgName !== 'string') {
    throw new Error(`Invalid 'setterArgName' key type.`);
  } else {
    propertyJson['setterArgName.gen'] = false;
  }
  fillName('setterArgName', propertyJson, setterArgNameLangToStyleMap);

  if (typeof propertyJson.setterMode === 'undefined') {
    propertyJson.setterMode = MethodMode.regular; // TODO: perhaps determine from base class(es)
    propertyJson['setterMode.gen'] = true;
  } else if (typeof propertyJson.setterMode !== 'string') {
    throw new Error(`Invalid 'setterMode' key type.`);
  } else {
    let found = false;
    for (const [key, value] of Object.entries(MethodMode)) {
      if (propertyJson.setterMode === value) {
        found = true;
        break;
      }
    }
    if (!found) {
      throw new Error(`Unknown 'setterMode' value: ${propertyJson.setterMode}`);
    }
    propertyJson['setterMode.gen'] = false;
  }

  if (typeof propertyJson.setterCustom === 'undefined') {
    const setterMode = getPropertySetterMode(propertyJson);
    propertyJson.setterCustom = setterMode === MethodMode.pureVirtual;
    propertyJson['setterCustom.gen'] = true;
  } else if (typeof propertyJson.setterCustom !== 'boolean') {
    throw new Error(`Invalid 'setterCustom' key type.`);
  } else {
    propertyJson['setterCustom.gen'] = false;
  }

  if (typeof propertyJson.setterVisibility === 'undefined') {
    propertyJson.setterVisibility = Visibility.public; // TODO: perhaps determine from base class(es)
    propertyJson['setterVisibility.gen'] = true;
  } else if (typeof propertyJson.setterVisibility !== 'string') {
    throw new Error(`Invalid 'setterVisibility' key type.`);
  } else {
    let found = false;
    for (const [key, value] of Object.entries(Visibility)) {
      if (propertyJson.setterVisibility === value) {
        found = true;
        break;
      }
    }
    if (!found) {
      throw new Error(`Unknown 'setterVisibility' value: ${propertyJson.setterVisibility}`);
    }
    propertyJson['setterVisibility.gen'] = false;
  }

  if (typeof propertyJson.hasMemberVar === 'undefined') {
    const getterCustom = getPropertyGetterCustom(propertyJson);
    const setterCustom = getPropertySetterCustom(propertyJson);
    propertyJson.hasMemberVar = !getterCustom || !setterCustom;
    propertyJson['hasMemberVar.gen'] = true;
  } else if (typeof propertyJson.hasMemberVar !== 'boolean') {
    throw new Error(`Invalid 'hasMemberVar' key type.`);
  } else {
    propertyJson['hasMemberVar.gen'] = false;
  }

  if (typeof propertyJson.defaultValue === 'undefined') {
    propertyJson.defaultValue = '';
    propertyJson['defaultValue.gen'] = true;
  } else if (typeof propertyJson.defaultValue !== 'string') {
    throw new Error(`Invalid 'defaultValue' key type.`);
  } else {
    propertyJson['defaultValue.gen'] = false;
  }

  if (typeof propertyJson.partOfIdentity === 'undefined') {
    propertyJson.partOfIdentity = !propertyJson.static;
    propertyJson['partOfIdentity.gen'] = true;
  } else if (typeof propertyJson.partOfIdentity !== 'boolean') {
    throw new Error(`Invalid 'partOfIdentity' key type.`);
  } else {
    propertyJson['partOfIdentity.gen'] = false;
  }
}

function fillProperties(interfaceJson, nameLangToStyleMap = defaultPropNameLangToStyleMap, getterNameLangToStyleMap = defaultGetterNameLangToStyleMap, setterNameLangToStyleMap = defaultSetterNameLangToStyleMap, setterArgNameLangToStyleMap = defaultSetterArgNameLangToStyleMap) {
  const nameKey = 'properties';
  const nameKeyGen = `${nameKey}.gen`;
  if (typeof interfaceJson[nameKey] === 'undefined') {
    interfaceJson[nameKey] = [];
    interfaceJson[nameKeyGen] = true;
    return;
  }
  if (!Array.isArray(interfaceJson[nameKey])) {
    throw new Error(`Invalid '${nameKey}' key type.`);
  }
  for (const propertyJson of interfaceJson[nameKey]) {
    fillProperty(interfaceJson, propertyJson, nameLangToStyleMap, getterNameLangToStyleMap, setterNameLangToStyleMap, setterArgNameLangToStyleMap);
  }
  interfaceJson[nameKeyGen] = false;
}

function fillCopyable(interfaceJson) {
  if (typeof interfaceJson.copyable === 'undefined') {
    interfaceJson.copyable = !getClassStatic(interfaceJson);
    interfaceJson['copyable.gen'] = true;
  } else if (typeof interfaceJson.copyable !== 'boolean') {
    throw new Error(`Invalid 'copyable' key type.`);
  } else {
    interfaceJson['copyable.gen'] = false;
  }
}

function fillMovable(interfaceJson) {
  if (typeof interfaceJson.movable === 'undefined') {
    interfaceJson.movable = !getClassStatic(interfaceJson);
    interfaceJson['movable.gen'] = true;
  } else if (typeof interfaceJson.movable !== 'boolean') {
    throw new Error(`Invalid 'movable' key type.`);
  } else {
    interfaceJson['movable.gen'] = false;
  }
}

function fillConstructorDefinition(constructorJson, defaultIfNotSet) {
  if (typeof constructorJson.definition === 'undefined') {
    constructorJson.definition = defaultIfNotSet;
    constructorJson['definition.gen'] = true;
  } else if (typeof constructorJson.definition !== 'string') {
    throw new Error(`Invalid 'definition' key type.`);
  } else {
    let found = false;
    for (const [key, value] of Object.entries(ConstructorDefinition)) {
      if (constructorJson.definition === value) {
        found = true;
        break;
      }
    }
    if (!found) {
      throw new Error(`Unknown 'definition' value: ${constructorJson.definition}`);
    }
    constructorJson['definition.gen'] = false;
  }
}

function fillConstructorVisibility(constructorJson, defaultIfNotSet) {
  if (typeof constructorJson.visibility === 'undefined') {
    constructorJson.visibility = defaultIfNotSet;
    constructorJson['visibility.gen'] = true;
  } else if (typeof constructorJson.visibility !== 'string') {
    throw new Error(`Invalid 'visibility' key type.`);
  } else {
    let found = false;
    for (const [key, value] of Object.entries(Visibility)) {
      if (constructorJson.visibility === value) {
        found = true;
        break;
      }
    }
    if (!found) {
      throw new Error(`Unknown 'visibility' value: ${constructorJson.visibility}`);
    }
    constructorJson['visibility.gen'] = false;
  }
}

function fillConstructorNoexcept(constructorJson, defaultIfNotSet) {
  if (typeof constructorJson.noexcept === 'undefined') {
    constructorJson.noexcept = defaultIfNotSet;
    constructorJson['noexcept.gen'] = true;
  } else if (typeof constructorJson.noexcept !== 'boolean') {
    throw new Error(`Invalid 'noexcept' key type.`);
  } else {
    constructorJson['noexcept.gen'] = false;
  }
}

let defaultFuncArgNameLangToStyleMap = {};
defaultFuncArgNameLangToStyleMap[Language.CXX] = NameStyle.camelBack;
defaultFuncArgNameLangToStyleMap[Language.C] = NameStyle.lower_case;
defaultFuncArgNameLangToStyleMap[Language.ObjC] = NameStyle.camelBack;
defaultFuncArgNameLangToStyleMap[Language.Swift] = NameStyle.camelBack;
defaultFuncArgNameLangToStyleMap[Language.JS] = NameStyle.camelBack;

function fillCopyOrMoveConstructorMainArgName(interfaceJson, constructorJson, funcArgNameLangToStyleMap = defaultFuncArgNameLangToStyleMap) {
  if (typeof constructorJson.mainArgName === 'undefined') {
    constructorJson.mainArgName = interfaceJson.name;
    constructorJson['mainArgName.gen'] = true;
  } else if (typeof constructorJson.mainArgName !== 'string') {
    throw new Error(`Invalid 'mainArgName' key type.`);
  } else {
    constructorJson['mainArgName.gen'] = false;
  }
  fillName('mainArgName', constructorJson, funcArgNameLangToStyleMap);
}

const FillConstructorInitializedPropertiesType = {
  parameterlessConstructor: 1,
  copyConstructor: 2,
  moveConstructor: 3
};

function fillConstructorInitializedProperties(interfaceJson, constructorJson, fillConstructorInitializedPropertiesType) {
  const nameKey = 'initializedProperties';
  const nameKeyGen = `${nameKey}.gen`;
  let result = {
    canBeNoexcept: true,
    canBeDefault: true
  };
  if (typeof constructorJson[nameKey] === 'undefined') {
    constructorJson[nameKey] = [];
    constructorJson[nameKeyGen] = true;
    for (const propertyJson of getProperties(interfaceJson)) {
      if (!getPropertyStatic(propertyJson) && getPropertyHasMemberVar(propertyJson)) {
        const type = propertyJson.type;
        if (!isPrimitiveType(type)) {
          result.canBeNoexcept = false;
        }
        let value = undefined;
        switch (fillConstructorInitializedPropertiesType) {
          case FillConstructorInitializedPropertiesType.parameterlessConstructor:
            {
              const defaultValue = getPropertyDefaultValue(propertyJson);
              value = defaultValue.length > 0 ? defaultValue : getTypeImplicitDefaultValue(type);
            }
            break;
          case FillConstructorInitializedPropertiesType.copyConstructor:
            value = getTypeMembVarCopyOp(type, '@');
            break;
          case FillConstructorInitializedPropertiesType.moveConstructor:
            value = getTypeMembVarMoveOp(type, '@');
            break;
          default:
            throw new Error(`Unhandled 'FillConstructorInitializedPropertiesType' value: ${fillConstructorInitializedPropertiesType}`);
        }
        const obj = {
          name: propertyJson.name,
          'name.gen': true,
          value: value,
          'value.gen': true,
          valuePlaceholder: '@',
          'valuePlaceholder.gen': true,
          initializeInBody: false,
          'initializeInBody.gen': true
        };
        constructorJson[nameKey].push(obj);
      }
    }
  } else if (!Array.isArray(constructorJson[nameKey])) {
    throw new Error(`Invalid '${nameKey}' key type.`);
  } else {
    constructorJson[nameKeyGen] = false;
    for (const initializedPropertyJson of constructorJson[nameKey]) {
      const name = initializedPropertyJson.name;
      initializedPropertyJson['name.gen'] = false;
      if (typeof name === 'undefined') {
        throw new Error(`Missing 'name' key.`);
      } else if (typeof name !== 'string') {
        throw new Error(`Invalid 'name' key type.`);
      }
      let foundPropertyJson = undefined;
      for (const propertyJson of getProperties(interfaceJson)) {
        if (propertyJson.name === name) {
          foundPropertyJson = propertyJson;
          break;
        }
      }
      if (foundPropertyJson === undefined) {
        throw new Error(`Property '${name}' does not exist.`);
      }
      if (getPropertyStatic(foundPropertyJson)) {
        throw new Error(`Property '${name}' is static.`);
      }
      if (!getPropertyHasMemberVar(foundPropertyJson)) {
        throw new Error(`Property '${name}' does not have a member variable.`);
      }
      const type = foundPropertyJson.type;
      if (!isPrimitiveType(type)) {
        result.canBeNoexcept = false;
      }
      if (typeof initializedPropertyJson.valuePlaceholder === 'undefined') {
        initializedPropertyJson['valuePlaceholder'] = '@';
        initializedPropertyJson['valuePlaceholder.gen'] = true;
      } else if (typeof initializedPropertyJson.valuePlaceholder !== 'string') {
        throw new Error(`Invalid 'valuePlaceholder' key type.`);
      } else {
        initializedPropertyJson['valuePlaceholder.gen'] = false;
      }
      if (typeof initializedPropertyJson.initializeInBody === 'undefined') {
        initializedPropertyJson['initializeInBody'] = false;
        initializedPropertyJson['initializeInBody.gen'] = true;
      } else if (typeof initializedPropertyJson.initializeInBody !== 'boolean') {
        throw new Error(`Invalid 'initializeInBody' key type.`);
      } else {
        initializedPropertyJson['initializeInBody.gen'] = false;
      }
      if (typeof initializedPropertyJson.value === 'undefined') {
        let value = undefined;
        switch (fillConstructorInitializedPropertiesType) {
          case FillConstructorInitializedPropertiesType.parameterlessConstructor:
            {
              const defaultValue = getPropertyDefaultValue(foundPropertyJson);
              value = defaultValue.length > 0 ? defaultValue : getTypeImplicitDefaultValue(type);
            }
            break;
          case FillConstructorInitializedPropertiesType.copyConstructor:
            value = getTypeMembVarCopyOp(type, initializedPropertyJson.valuePlaceholder);
            break;
          case FillConstructorInitializedPropertiesType.moveConstructor:
            value = getTypeMembVarMoveOp(type, initializedPropertyJson.valuePlaceholder);
            break;
          default:
            throw new Error(`Unhandled 'FillConstructorInitializedPropertiesType' value: ${fillConstructorInitializedPropertiesType}`);
        }
        initializedPropertyJson.value = value;
        initializedPropertyJson['value.gen'] = true;
      } else if (typeof initializedPropertyJson.value !== 'string') {
        throw new Error(`Invalid 'value' key type.`);
      } else {
        initializedPropertyJson['value.gen'] = false;
      }
    }
  }
  for (const initializedPropertyJson of constructorJson[nameKey]) {
    let foundPropertyJson = undefined;
    for (const propertyJson of getProperties(interfaceJson)) {
      if (propertyJson.name === initializedPropertyJson.name) {
        foundPropertyJson = propertyJson;
        break;
      }
    }
    switch (fillConstructorInitializedPropertiesType) {
      case FillConstructorInitializedPropertiesType.parameterlessConstructor:
        if (initializedPropertyJson.value.length > 0) {
          result.canBeDefault = false;
        }
        break;
      case FillConstructorInitializedPropertiesType.copyConstructor:
        if (initializedPropertyJson.value.length > 0 && initializedPropertyJson.value !== getTypeMembVarCopyOp(foundPropertyJson.type, initializedPropertyJson.valuePlaceholder)) {
          result.canBeDefault = false;
        } else if (isClassPtrType(foundPropertyJson.type) || isClassPtrRefType(foundPropertyJson.type) || isClassPtrMixType(foundPropertyJson.type)) {
          result.canBeDefault = false;
        } else if (isArrayPtrType(foundPropertyJson.type) || isArrayPtrRefType(foundPropertyJson.type) || isArrayPtrMixType(foundPropertyJson.type)) {
          result.canBeDefault = false;
        }
        break;
      case FillConstructorInitializedPropertiesType.moveConstructor:
        if (initializedPropertyJson.value.length > 0 && initializedPropertyJson.value !== getTypeMembVarMoveOp(foundPropertyJson.type, initializedPropertyJson.valuePlaceholder)) {
          result.canBeDefault = false;
        }
        break;
      default:
        throw new Error(`Unhandled 'FillConstructorInitializedPropertiesType' value: ${fillConstructorInitializedPropertiesType}`);
    }
  }
  return result;
}

function fillConstructorCodeGeneric(nameKey, constructorJson, defaultCode = []) {
  const nameKeyGen = `${nameKey}.gen`;
  let result = {
    canBeDefault: true
  };
  if (typeof constructorJson[nameKey] === 'undefined') {
    constructorJson[nameKey] = defaultCode;
    constructorJson[nameKeyGen] = true;
  } else if (!Array.isArray(constructorJson[nameKey])) {
    throw new Error(`Invalid '${nameKey}' key type.`);
  } else {
    for (const line of constructorJson[nameKey]) {
      result.canBeDefault = false;
      if (typeof line !== 'string') {
        throw new Error(`Invalid '${nameKey}' key type.`);
      }
    }
    constructorJson[nameKeyGen] = false;
  }
  return result;
}

function fillConstructorCodeFront(constructorJson) {
  return fillConstructorCodeGeneric('codeFront', constructorJson);
}

function fillConstructorCodeBack(constructorJson) {
  return fillConstructorCodeGeneric('codeBack', constructorJson);
}

function fillConstructorCode(constructorJson) {
  const resultFront = fillConstructorCodeFront(constructorJson);
  const resultBack = fillConstructorCodeBack(constructorJson);
  return {
    canBeDefault: resultFront.canBeDefault && resultBack.canBeDefault
  };
}

function fillConstructorOperatorCodeFront(constructorJson) {
  return fillConstructorCodeGeneric('operatorCodeFront', constructorJson, constructorJson['codeFront']);
}

function fillConstructorOperatorCodeBack(constructorJson) {
  return fillConstructorCodeGeneric('operatorCodeBack', constructorJson, constructorJson['codeBack']);
}

function fillConstructorOperatorCode(constructorJson) {
  const resultFront = fillConstructorOperatorCodeFront(constructorJson);
  const resultBack = fillConstructorOperatorCodeBack(constructorJson);
  return {
    canBeDefault: resultFront.canBeDefault && resultBack.canBeDefault
  };
}

function fillConstructorCustom(constructorJson) {
  if (typeof constructorJson.custom === 'undefined') {
    constructorJson.custom = false;
    constructorJson['custom.gen'] = true;
  } else if (typeof constructorJson.custom !== 'boolean') {
    throw new Error(`Invalid 'custom' key type.`);
  } else {
    constructorJson['custom.gen'] = false;
  }
}

function fillConstructorCustomOperator(constructorJson) {
  if (typeof constructorJson.customOperator === 'undefined') {
    constructorJson.customOperator = constructorJson.custom;
    constructorJson['customOperator.gen'] = true;
  } else if (typeof constructorJson.customOperator !== 'boolean') {
    throw new Error(`Invalid 'customOperator' key type.`);
  } else {
    constructorJson['customOperator.gen'] = false;
  }
}

function fillParameterlessConstructor(interfaceJson) {
  const nameKey = 'parameterlessConstructor';
  const nameKeyGen = `${nameKey}.gen`;
  if (typeof interfaceJson[nameKey] === 'undefined') {
    interfaceJson[nameKey] = {};
    interfaceJson[nameKeyGen] = true;
  } else {
    interfaceJson[nameKeyGen] = false;
  }
  if (typeof interfaceJson[nameKey] !== 'object') {
    throw new Error(`Invalid '${nameKey}' key type.`);
  }
  const fcipResult = fillConstructorInitializedProperties(interfaceJson, interfaceJson[nameKey], FillConstructorInitializedPropertiesType.parameterlessConstructor);
  const fccResult = fillConstructorCode(interfaceJson[nameKey]);
  const classStatic = getClassStatic(interfaceJson);
  const canBeDefault = fcipResult.canBeDefault && fccResult.canBeDefault;
  fillConstructorDefinition(interfaceJson[nameKey], classStatic ? ConstructorDefinition.deleted : (canBeDefault ? ConstructorDefinition.default : ConstructorDefinition.defined));
  fillConstructorVisibility(interfaceJson[nameKey], classStatic ? Visibility.private : Visibility.public);
  const canBeNoexcept = fcipResult.canBeNoexcept;
  if (canBeNoexcept) {
    fillConstructorNoexcept(interfaceJson[nameKey], getConstructorDefinition(interfaceJson[nameKey]) !== ConstructorDefinition.deleted);
  } else {
    fillConstructorNoexcept(interfaceJson[nameKey], false);
  }
  fillConstructorCustom(interfaceJson[nameKey]);
}

function fillCopyConstructor(interfaceJson, funcArgNameLangToStyleMap = defaultFuncArgNameLangToStyleMap) {
  const nameKey = 'copyConstructor';
  const nameKeyGen = `${nameKey}.gen`;
  if (typeof interfaceJson[nameKey] === 'undefined') {
    interfaceJson[nameKey] = {};
    interfaceJson[nameKeyGen] = true;
  } else {
    interfaceJson[nameKeyGen] = false;
  }
  if (typeof interfaceJson[nameKey] !== 'object') {
    throw new Error(`Invalid '${nameKey}' key type.`);
  }
  fillCopyOrMoveConstructorMainArgName(interfaceJson, interfaceJson[nameKey], funcArgNameLangToStyleMap);
  const fcipResult = fillConstructorInitializedProperties(interfaceJson, interfaceJson[nameKey], FillConstructorInitializedPropertiesType.copyConstructor);
  const fccResult = fillConstructorCode(interfaceJson[nameKey]);
  const fcocResult = fillConstructorOperatorCode(interfaceJson[nameKey]);
  const classStatic = getClassStatic(interfaceJson);
  const classCopyable = getClassCopyable(interfaceJson);
  const canBeDefault = fcipResult.canBeDefault && fccResult.canBeDefault && fcocResult.canBeDefault;
  if (classCopyable) {
    fillConstructorDefinition(interfaceJson[nameKey], canBeDefault ? ConstructorDefinition.default : ConstructorDefinition.defined);
  } else {
    fillConstructorDefinition(interfaceJson[nameKey], classStatic ? ConstructorDefinition.omitted : ConstructorDefinition.deleted);
  }
  fillConstructorVisibility(interfaceJson[nameKey], Visibility.public);
  const canBeNoexcept = fcipResult.canBeNoexcept;
  if (canBeNoexcept) {
    fillConstructorNoexcept(interfaceJson[nameKey], getConstructorDefinition(interfaceJson[nameKey]) !== ConstructorDefinition.deleted);
  } else {
    fillConstructorNoexcept(interfaceJson[nameKey], false);
  }
  fillConstructorCustom(interfaceJson[nameKey]);
  fillConstructorCustomOperator(interfaceJson[nameKey]);
}

function fillMoveConstructor(interfaceJson, funcArgNameLangToStyleMap = defaultFuncArgNameLangToStyleMap) {
  const nameKey = 'moveConstructor';
  const nameKeyGen = `${nameKey}.gen`;
  if (typeof interfaceJson[nameKey] === 'undefined') {
    interfaceJson[nameKey] = {};
    interfaceJson[nameKeyGen] = true;
  } else {
    interfaceJson[nameKeyGen] = false;
  }
  if (typeof interfaceJson[nameKey] !== 'object') {
    throw new Error(`Invalid '${nameKey}' key type.`);
  }
  fillCopyOrMoveConstructorMainArgName(interfaceJson, interfaceJson[nameKey], funcArgNameLangToStyleMap);
  const fcipResult = fillConstructorInitializedProperties(interfaceJson, interfaceJson[nameKey], FillConstructorInitializedPropertiesType.moveConstructor);
  const fccResult = fillConstructorCode(interfaceJson[nameKey]);
  const fcocResult = fillConstructorOperatorCode(interfaceJson[nameKey]);
  const classStatic = getClassStatic(interfaceJson);
  const classMovable = getClassMovable(interfaceJson);
  const canBeDefault = fcipResult.canBeDefault && fccResult.canBeDefault && fcocResult.canBeDefault;
  if (classMovable) {
    fillConstructorDefinition(interfaceJson[nameKey], canBeDefault ? ConstructorDefinition.default : ConstructorDefinition.defined);
  } else {
    fillConstructorDefinition(interfaceJson[nameKey], classStatic ? ConstructorDefinition.omitted : ConstructorDefinition.deleted);
  }
  fillConstructorVisibility(interfaceJson[nameKey], Visibility.public);
  const canBeNoexcept = fcipResult.canBeNoexcept;
  if (canBeNoexcept) {
    fillConstructorNoexcept(interfaceJson[nameKey], getConstructorDefinition(interfaceJson[nameKey]) !== ConstructorDefinition.deleted);
  } else {
    fillConstructorNoexcept(interfaceJson[nameKey], false);
  }
  fillConstructorCustom(interfaceJson[nameKey]);
  fillConstructorCustomOperator(interfaceJson[nameKey]);
}

function fillDestructorVirtual(interfaceJson, destructorJson) {
  if (typeof destructorJson.virtual === 'undefined') {
    destructorJson.virtual = false; // TODO: perhaps determine from derived class(es)
    destructorJson['virtual.gen'] = true;
  } else if (typeof destructorJson.virtual !== 'boolean') {
    throw new Error(`Invalid 'virtual' key type.`);
  } else {
    destructorJson['virtual.gen'] = false;
  }
}

function fillDestructorCode(destructorJson) {
  fillConstructorCodeGeneric('code', destructorJson);
}

function fillDestructorDefinition(destructorJson, defaultIfNotSet) {
  if (typeof destructorJson.definition === 'undefined') {
    destructorJson.definition = defaultIfNotSet;
    destructorJson['definition.gen'] = true;
  } else if (typeof destructorJson.definition !== 'string') {
    throw new Error(`Invalid 'definition' key type.`);
  } else {
    let found = false;
    for (const [key, value] of Object.entries(DestructorDefinition)) {
      if (destructorJson.definition === value) {
        found = true;
        break;
      }
    }
    if (!found) {
      throw new Error(`Unknown 'definition' value: ${destructorJson.definition}`);
    }
    destructorJson['definition.gen'] = false;
  }
}

function fillDestructorVisibility(destructorJson, defaultIfNotSet) {
  if (typeof destructorJson.visibility === 'undefined') {
    destructorJson.visibility = defaultIfNotSet;
    destructorJson['visibility.gen'] = true;
  } else if (typeof destructorJson.visibility !== 'string') {
    throw new Error(`Invalid 'visibility' key type.`);
  } else {
    let found = false;
    for (const [key, value] of Object.entries(Visibility)) {
      if (destructorJson.visibility === value) {
        found = true;
        break;
      }
    }
    if (!found) {
      throw new Error(`Unknown 'visibility' value: ${destructorJson.visibility}`);
    }
    destructorJson['visibility.gen'] = false;
  }
}

function fillDestructorCustom(destructorJson) {
  if (typeof destructorJson.custom === 'undefined') {
    destructorJson.custom = false;
    destructorJson['custom.gen'] = true;
  } else if (typeof destructorJson.custom !== 'boolean') {
    throw new Error(`Invalid 'custom' key type.`);
  } else {
    destructorJson['custom.gen'] = false;
  }
}

function fillDestructor(interfaceJson) {
  const nameKey = 'destructor';
  const nameKeyGen = `${nameKey}.gen`;
  if (typeof interfaceJson[nameKey] === 'undefined') {
    interfaceJson[nameKey] = {};
    interfaceJson[nameKeyGen] = true;
  } else {
    interfaceJson[nameKeyGen] = false;
  }
  if (typeof interfaceJson[nameKey] !== 'object') {
    throw new Error(`Invalid '${nameKey}' key type.`);
  }
  fillDestructorVirtual(interfaceJson, interfaceJson[nameKey]);
  fillDestructorCode(interfaceJson[nameKey]);
  fillDestructorDefinition(interfaceJson[nameKey], interfaceJson[nameKey].code.length > 0 ? DestructorDefinition.defined : DestructorDefinition.default);
  fillDestructorVisibility(interfaceJson[nameKey], Visibility.public);
  fillDestructorCustom(interfaceJson[nameKey]);
}

function makeEqualityOperatorComparedProperties(interfaceJson) {
  let comparedProperties = [];
  for (const propertyJson of getProperties(interfaceJson)) {
    if (!getPropertyStatic(propertyJson) && propertyJson.partOfIdentity) {
      let useGetter = undefined;
      if (getPropertyHasMemberVar(propertyJson)) {
        useGetter = false;
      } else if (propertyJson.hasGetter) {
        useGetter = true;
      } else {
        continue;
      }
      comparedProperties.push({
        name: propertyJson.name,
        'name.gen': true,
        useGetter: useGetter,
        'useGetter.gen': true,
        comparator: getTypePropEqOp(propertyJson.type, '@', '$'),
        'comparator.gen': true,
        comparatorClassInstancePlaceholder: '@',
        'comparatorClassInstancePlaceholder.gen': true,
        comparatorPropertyPlaceholder: '$',
        'comparatorPropertyPlaceholder.gen': true
      });
    }
  }
  return comparedProperties;
}

function fillEqualityOperator(interfaceJson) {
  const nameKey = 'equalityOperator';
  const nameKeyGen = `${nameKey}.gen`;
  if (typeof interfaceJson[nameKey] === 'undefined') {
    interfaceJson[nameKeyGen] = true;
    interfaceJson[nameKey] = {
      defined: !getClassStatic(interfaceJson),
      'defined.gen': true,
      comparePointers: !getClassCopyable(interfaceJson),
      'comparePointers.gen': true,
      comparedProperties: makeEqualityOperatorComparedProperties(interfaceJson),
      'comparedProperties.gen': true,
      custom: false,
      'custom.gen': true,
      customInequality: false,
      'customInequality.gen': true
    };
    return;
  } else if (typeof interfaceJson[nameKey] !== 'object') {
    throw new Error(`Invalid '${nameKey}' key type.`);
  }
  interfaceJson[nameKeyGen] = false;

  if (typeof interfaceJson[nameKey].defined === 'undefined') {
    interfaceJson[nameKey].defined = !getClassStatic(interfaceJson);
    interfaceJson[nameKey]['defined.gen'] = true;
  } else if (typeof interfaceJson[nameKey].defined !== 'boolean') {
    throw new Error(`Invalid 'defined' key type.`);
  } else {
    interfaceJson[nameKey]['defined.gen'] = false;
  }

  if (typeof interfaceJson[nameKey].comparePointers === 'undefined') {
    interfaceJson[nameKey].comparePointers = !getClassCopyable(interfaceJson);
    interfaceJson[nameKey]['comparePointers.gen'] = true;
  } else if (typeof interfaceJson[nameKey].comparePointers !== 'boolean') {
    throw new Error(`Invalid 'comparePointers' key type.`);
  } else {
    interfaceJson[nameKey]['comparePointers.gen'] = false;
  }

  if (typeof interfaceJson[nameKey].comparedProperties === 'undefined') {
    interfaceJson[nameKey].comparedProperties = makeEqualityOperatorComparedProperties(interfaceJson);
    interfaceJson[nameKey]['comparedProperties.gen'] = true;
  } else if (!Array.isArray(interfaceJson[nameKey].comparedProperties)) {
    throw new Error(`Invalid 'comparedProperties' key type.`);
  } else {
    interfaceJson[nameKey]['comparedProperties.gen'] = false;
    for (const comparedPropertyJson of interfaceJson[nameKey].comparedProperties) {
      let foundPropertyJson = undefined;
      for (const propertyJson of getProperties(interfaceJson)) {
        if (propertyJson.name === comparedPropertyJson.name) {
          foundPropertyJson = propertyJson;
          break;
        }
      }
      if (foundPropertyJson === undefined) {
        throw new Error(`Property '${comparedPropertyJson.name}' does not exist.`);
      }
      if (getPropertyStatic(foundPropertyJson)) {
        throw new Error(`Property '${comparedPropertyJson.name}' is static.`);
      }
      comparedPropertyJson['name.gen'] = false;

      if (typeof comparedPropertyJson.useGetter === 'undefined') {
        let useGetter = undefined;
        if (getPropertyHasMemberVar(foundPropertyJson)) {
          useGetter = false;
        } else if (foundPropertyJson.hasGetter) {
          useGetter = true;
        } else {
          throw new Error(`Property '${comparedPropertyJson.name}' does not have a member variable or a getter method.`);
        }
        comparedPropertyJson.useGetter = useGetter;
        comparedPropertyJson['useGetter.gen'] = true;
      } else if (typeof comparedPropertyJson.useGetter !== 'boolean') {
        throw new Error(`Invalid 'useGetter' key type.`);
      } else {
        comparedPropertyJson['useGetter.gen'] = false;
      }

      if (typeof comparedPropertyJson.comparatorClassInstancePlaceholder === 'undefined') {
        comparedPropertyJson.comparatorClassInstancePlaceholder = '@';
        comparedPropertyJson['comparatorClassInstancePlaceholder.gen'] = true;
      } else if (typeof comparedPropertyJson.comparatorClassInstancePlaceholder !== 'string') {
        throw new Error(`Invalid 'comparatorClassInstancePlaceholder' key type.`);
      } else {
        comparedPropertyJson['comparatorClassInstancePlaceholder.gen'] = false;
      }

      if (typeof comparedPropertyJson.comparatorPropertyPlaceholder === 'undefined') {
        comparedPropertyJson.comparatorPropertyPlaceholder = '$';
        comparedPropertyJson['comparatorPropertyPlaceholder.gen'] = true;
      } else if (typeof comparedPropertyJson.comparatorPropertyPlaceholder !== 'string') {
        throw new Error(`Invalid 'comparatorPropertyPlaceholder' key type.`);
      } else {
        comparedPropertyJson['comparatorPropertyPlaceholder.gen'] = false;
      }

      if (typeof comparedPropertyJson.comparator === 'undefined') {
        comparedPropertyJson.comparator = getTypePropEqOp(foundPropertyJson.type, comparedPropertyJson.comparatorClassInstancePlaceholder, comparedPropertyJson.comparatorPropertyPlaceholder);
        comparedPropertyJson['comparator.gen'] = true;
      } else if (typeof comparedPropertyJson.comparator !== 'string') {
        throw new Error(`Invalid 'comparator' key type.`);
      } else {
        comparedPropertyJson['comparator.gen'] = false;
      }
    }
  }

  if (typeof interfaceJson[nameKey].custom === 'undefined') {
    interfaceJson[nameKey].custom = false;
    interfaceJson[nameKey]['custom.gen'] = true;
  } else if (typeof interfaceJson[nameKey].custom !== 'boolean') {
    throw new Error(`Invalid 'custom' key type.`);
  } else {
    interfaceJson[nameKey]['custom.gen'] = false;
  }

  if (typeof interfaceJson[nameKey].customInequality === 'undefined') {
    interfaceJson[nameKey].customInequality = false;
    interfaceJson[nameKey]['customInequality.gen'] = true;
  } else if (typeof interfaceJson[nameKey].customInequality !== 'boolean') {
    throw new Error(`Invalid 'customInequality' key type.`);
  } else {
    interfaceJson[nameKey]['customInequality.gen'] = false;
  }
}

function makeHashOperatorHashedProperties(interfaceJson) {
  let hashedProperties = [];
  for (const comparedPropertyJson of interfaceJson.equalityOperator.comparedProperties) {
    let foundPropertyJson = undefined;
    for (const propertyJson of getProperties(interfaceJson)) {
      if (propertyJson.name === comparedPropertyJson.name) {
        foundPropertyJson = propertyJson;
        break;
      }
    }
    hashedProperties.push({
      name: comparedPropertyJson.name,
      'name.gen': true,
      useGetter: comparedPropertyJson.useGetter,
      'useGetter.gen': true,
      hasher: getTypePropHasher(foundPropertyJson.type, '@'),
      'hasher.gen': true,
      hasherPlaceholder: '@',
      'hasherPlaceholder.gen': true
    });
  }
  return hashedProperties;
}

function fillHashOperator(interfaceJson) {
  const nameKey = 'hashOperator';
  const nameKeyGen = `${nameKey}.gen`;
  if (typeof interfaceJson[nameKey] === 'undefined') {
    interfaceJson[nameKeyGen] = true;
    interfaceJson[nameKey] = {
      defined: interfaceJson.equalityOperator.defined,
      'defined.gen': true,
      usePointerAsHash: interfaceJson.equalityOperator.comparePointers,
      'usePointerAsHash.gen': true,
      hashedProperties: makeHashOperatorHashedProperties(interfaceJson),
      'hashedProperties.gen': true,
      custom: false,
      'custom.gen': true
    };
    return;
  } else if (typeof interfaceJson[nameKey] !== 'object') {
    throw new Error(`Invalid '${nameKey}' key type.`);
  }
  interfaceJson[nameKeyGen] = false;

  if (typeof interfaceJson[nameKey].defined === 'undefined') {
    interfaceJson[nameKey].defined = interfaceJson.equalityOperator.defined;
    interfaceJson[nameKey]['defined.gen'] = true;
  } else if (typeof interfaceJson[nameKey].defined !== 'boolean') {
    throw new Error(`Invalid 'defined' key type.`);
  } else {
    interfaceJson[nameKey]['defined.gen'] = false;
  }

  if (typeof interfaceJson[nameKey].usePointerAsHash === 'undefined') {
    interfaceJson[nameKey].usePointerAsHash = interfaceJson.equalityOperator.comparePointers;
    interfaceJson[nameKey]['usePointerAsHash.gen'] = true;
  } else if (typeof interfaceJson[nameKey].usePointerAsHash !== 'boolean') {
    throw new Error(`Invalid 'usePointerAsHash' key type.`);
  } else {
    interfaceJson[nameKey]['usePointerAsHash.gen'] = false;
  }

  if (typeof interfaceJson[nameKey].hashedProperties === 'undefined') {
    interfaceJson[nameKey].hashedProperties = makeHashOperatorHashedProperties(interfaceJson);
    interfaceJson[nameKey]['hashedProperties.gen'] = true;
  } else if (!Array.isArray(interfaceJson[nameKey].hashedProperties)) {
    throw new Error(`Invalid 'hashedProperties' key type.`);
  } else {
    interfaceJson[nameKey]['hashedProperties.gen'] = false;
    for (const hashedPropertyJson of interfaceJson[nameKey].hashedProperties) {
      let foundPropertyJson = undefined;
      for (const propertyJson of getProperties(interfaceJson)) {
        if (propertyJson.name === hashedPropertyJson.name) {
          foundPropertyJson = propertyJson;
          break;
        }
      }
      if (foundPropertyJson === undefined) {
        throw new Error(`Property '${hashedPropertyJson.name}' does not exist.`);
      }
      if (getPropertyStatic(foundPropertyJson)) {
        throw new Error(`Property '${hashedPropertyJson.name}' is static.`);
      }
      hashedPropertyJson['name.gen'] = false;

      let foundComparedPropertyJson = undefined;
      for (const comparedPropertyJson of interfaceJson.equalityOperator.comparedProperties) {
        if (comparedPropertyJson.name === hashedPropertyJson.name) {
          foundComparedPropertyJson = comparedPropertyJson;
          break;
        }
      }

      if (typeof hashedPropertyJson.useGetter === 'undefined') {
        let useGetter = undefined;
        if (typeof foundComparedPropertyJson !== 'undefined') {
          useGetter = foundComparedPropertyJson.useGetter;
        } else if (getPropertyHasMemberVar(foundPropertyJson)) {
          useGetter = false;
        } else if (foundPropertyJson.hasGetter) {
          useGetter = true;
        } else {
          throw new Error(`Property '${hashedPropertyJson.name}' does not have a member variable or a getter method.`);
        }
        hashedPropertyJson.useGetter = useGetter;
        hashedPropertyJson['useGetter.gen'] = true;
      } else if (typeof hashedPropertyJson.useGetter !== 'boolean') {
        throw new Error(`Invalid 'useGetter' key type.`);
      } else {
        hashedPropertyJson['useGetter.gen'] = false;
      }

      if (typeof hashedPropertyJson.hasherPlaceholder === 'undefined') {
        hashedPropertyJson.hasherPlaceholder = '@';
        hashedPropertyJson['hasherPlaceholder.gen'] = true;
      } else if (typeof hashedPropertyJson.hasherPlaceholder !== 'string') {
        throw new Error(`Invalid 'hasherPlaceholder' key type.`);
      } else {
        hashedPropertyJson['hasherPlaceholder.gen'] = false;
      }

      if (typeof hashedPropertyJson.hasher === 'undefined') {
        hashedPropertyJson.hasher = getTypePropHasher(foundPropertyJson.type, hashedPropertyJson.hasherPlaceholder);
        hashedPropertyJson['hasher.gen'] = true;
      } else if (typeof hashedPropertyJson.hasher !== 'string') {
        throw new Error(`Invalid 'hasher' key type.`);
      } else {
        hashedPropertyJson['hasher.gen'] = false;
      }
    }
  }

  if (typeof interfaceJson[nameKey].custom === 'undefined') {
    interfaceJson[nameKey].custom = interfaceJson.equalityOperator.custom;
    interfaceJson[nameKey]['custom.gen'] = true;
  } else if (typeof interfaceJson[nameKey].custom !== 'boolean') {
    throw new Error(`Invalid 'custom' key type.`);
  } else {
    interfaceJson[nameKey]['custom.gen'] = false;
  }
}

function fillToStringOperator(interfaceJson) {
  const nameKey = 'toStringOperator';
  const nameKeyGen = `${nameKey}.gen`;
  if (typeof interfaceJson[nameKey] === 'undefined') {
    interfaceJson[nameKeyGen] = true;
    interfaceJson[nameKey] = {
      defined: !interfaceJson.static,
      'defined.gen': true,
      custom: false,
      'custom.gen': true
    };
    return;
  } else if (typeof interfaceJson[nameKey] !== 'object') {
    throw new Error(`Invalid '${nameKey}' key type.`);
  }
  interfaceJson[nameKeyGen] = false;

  if (typeof interfaceJson[nameKey].defined === 'undefined') {
    interfaceJson[nameKey].defined = !interfaceJson.static;
    interfaceJson[nameKey]['defined.gen'] = true;
  } else if (typeof interfaceJson[nameKey].defined !== 'boolean') {
    throw new Error(`Invalid 'defined' key type.`);
  } else {
    interfaceJson[nameKey]['defined.gen'] = false;
  }

  if (typeof interfaceJson[nameKey].custom === 'undefined') {
    interfaceJson[nameKey].custom = false;
    interfaceJson[nameKey]['custom.gen'] = true;
  } else if (typeof interfaceJson[nameKey].custom !== 'boolean') {
    throw new Error(`Invalid 'custom' key type.`);
  } else {
    interfaceJson[nameKey]['custom.gen'] = false;
  }
}

function fillCGenerateFuncAliasDefines(interfaceJson) {
  if (typeof interfaceJson['c.generateFuncAliasDefines'] === 'undefined') {
    interfaceJson['c.generateFuncAliasDefines'] = true;
    interfaceJson['c.generateFuncAliasDefines.gen'] = true;
  } else if (typeof interfaceJson['c.generateFuncAliasDefines'] !== 'boolean') {
    throw new Error(`Invalid 'c.generateFuncAliasDefines' key type.`);
  } else {
    interfaceJson['c.generateFuncAliasDefines.gen'] = false;
  }
}

function writeFileContentIfDifferent(filePath, content) {
  if (fs.existsSync(filePath)) {
    if (!fs.statSync(filePath).isFile()) {
      throw new Error(`Item at '${filePath}' path is not a file.`);
    }
    if (fs.readFileSync(filePath, 'utf8') === content) {
      return;
    }
  }
  fs.writeFileSync(filePath, content, 'utf8');
}

module.exports = {
  NameStyle,
  lower_case: NameStyle.lower_case,
  UPPER_CASE: NameStyle.UPPER_CASE,
  camelBack: NameStyle.camelBack,
  CamelCase: NameStyle.CamelCase,
  camel_Snake_Back: NameStyle.camel_Snake_Back,
  Camel_Snake_Case: NameStyle.Camel_Snake_Case,
  Leading_upper_snake_case: NameStyle.Leading_upper_snake_case,
  NameStylePretty,
  Language,
  CXX: Language.CXX,
  C: Language.C,
  ObjC: Language.ObjC,
  Swift: Language.Swift,
  JS: Language.JS,
  LanguagePretty,
  MethodMode,
  Visibility,
  ConstructorDefinition,
  DestructorDefinition,
  getName,
  getStyleName,
  getLangName,
  defaultEnumNameLangToTransformationMap,
  defaultEnumConstantNameLangToTransformationMap,
  defaultClassNameLangToTransformationMap,
  getLangTransformedName,
  getLangEnumName,
  getLangEnumConstantName,
  getLangClassName,
  getClassStatic,
  getClassFinal,
  getClassCopyable,
  getClassMovable,
  getClassParameterlessConstructor,
  getClassCopyConstructor,
  getClassMoveConstructor,
  getConstructorDefinition,
  getConstructorVisibility,
  getConstructorNoexcept,
  defaultFuncArgNameLangToTransformationMap,
  getCopyOrMoveConstructorMainArgName,
  getConstructorInitializedProperties,
  getConstructorCodeFront,
  getConstructorCodeBack,
  getConstructorOperatorCodeFront,
  getConstructorOperatorCodeBack,
  getConstructorCustom,
  getConstructorCustomOperator,
  getDestructorVirtual,
  getDestructorCode,
  getDestructorDefinition,
  getDestructorVisibility,
  getDestructorCustom,
  getLangAliases,
  getHeaderGuardName,
  defaultPropNameLangToTransformationMap,
  getPropertyName,
  getFloatType,
  getDoubleType,
  getIntType,
  getBooleanType,
  getStringType,
  TypeFor,
  getStringRefType,
  getStringMixType,
  getEnumType,
  getClassType,
  getClassRefType,
  getClassMixType,
  getClassPtrType,
  getClassPtrRefType,
  getClassPtrMixType,
  wrapInArrayType,
  wrapInArrayRefType,
  wrapInArrayMixType,
  getArrayType,
  getArrayRefType,
  getArrayMixType,
  getArrayPtrType,
  getArrayPtrRefType,
  getArrayPtrMixType,
  getPropertyType,
  getPropertyTypeForGetter,
  getPropertyTypeForSetter,
  getPropertyGetterMode,
  getPropertySetterMode,
  getPropertyGetterCustom,
  getPropertySetterCustom,
  getPropertyGetterVisibility,
  getPropertySetterVisibility,
  getPropertyHasMemberVar,
  getPropertyDefaultValue,
  getPropertyStatic,
  isIntType,
  isEnumType,
  isClassType,
  isClassRefType,
  isClassMixType,
  isClassPtrType,
  isClassPtrRefType,
  isClassPtrMixType,
  isClassOfAnyType,
  isArrayType,
  isArrayRefType,
  isArrayMixType,
  isArrayPtrType,
  isArrayPtrRefType,
  isArrayPtrMixType,
  isArrayOfAnyType,
  isPrimitiveType,
  getTypeImplicitDefaultValue,
  getTypeMembVarCopyOp,
  getTypeMembVarMoveOp,
  getTypePropEqOp,
  getTypePropHasher,
  defaultPropGetterNameLangToTransformationMap,
  getPropertyGetterName,
  defaultPropSetterNameLangToTransformationMap,
  getPropertySetterName,
  defaultPropSetterArgNameLangToTransformationMap,
  getPropertySetterArgName,
  getProperties,
  convertNameStyle,
  defaultEnumNameLangToStyleMap,
  defaultEnumConstantNameLangToStyleMap,
  defaultClassNameLangToStyleMap,
  fillName,
  fillEnumName,
  fillEnumType,
  fillEnumConstantName,
  fillEnumConstants,
  fillClassName,
  fillClassStatic,
  fillClassFinal,
  fillAliases,
  fillHeaderGuardName,
  defaultPropNameLangToStyleMap,
  defaultGetterNameLangToStyleMap,
  defaultSetterNameLangToStyleMap,
  defaultSetterArgNameLangToStyleMap,
  fillProperty,
  fillProperties,
  fillCopyable,
  fillMovable,
  fillConstructorDefinition,
  fillConstructorVisibility,
  fillConstructorNoexcept,
  defaultFuncArgNameLangToStyleMap,
  fillCopyOrMoveConstructorMainArgName,
  FillConstructorInitializedPropertiesType,
  fillConstructorInitializedProperties,
  fillConstructorCodeGeneric,
  fillConstructorCodeFront,
  fillConstructorCodeBack,
  fillConstructorCode,
  fillConstructorOperatorCodeFront,
  fillConstructorOperatorCodeBack,
  fillConstructorOperatorCode,
  fillConstructorCustom,
  fillConstructorCustomOperator,
  fillParameterlessConstructor,
  fillCopyConstructor,
  fillMoveConstructor,
  fillDestructorVirtual,
  fillDestructorCode,
  fillDestructorDefinition,
  fillDestructorVisibility,
  fillDestructorCustom,
  fillDestructor,
  makeEqualityOperatorComparedProperties,
  fillEqualityOperator,
  makeHashOperatorHashedProperties,
  fillHashOperator,
  fillToStringOperator,
  fillCGenerateFuncAliasDefines,
  writeFileContentIfDifferent
};
