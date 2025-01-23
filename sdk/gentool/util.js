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
      return 'number';
    default:
      throw new Error(`Unknown language: ${language}`);
  }
}

function getPropertyType(propertyJson, language) {
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
  switch (propertyJson[key]) {
    case 'float':
      return getFloatType(language);
    case 'double':
      return getDoubleType(language);
    default:
      throw new Error(`Unknown type: ${propertyJson[key]}`);
  }
}

function getPropertyTypeForGetter(propertyJson, language) {
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
  switch (propertyJson[key]) {
    case 'float':
      return getFloatType(language);
    case 'double':
      return getDoubleType(language);
    default:
      throw new Error(`Unknown type: ${propertyJson[key]}`);
  }
}

function getPropertyTypeForSetter(propertyJson, language) {
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
  switch (propertyJson[key]) {
    case 'float':
      return getFloatType(language);
    case 'double':
      return getDoubleType(language);
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

function isPrimitiveType(type) {
  if (isIntType(type)) {
    return true;
  }
  switch (type) {
    case 'float':
    case 'double':
      return true;
    default:
      return false;
  }
}

function getTypeImplicitDefaultValue(type) {
  if (isIntType(type)) {
    return '0';
  }
  switch (type) {
    case 'float':
      return '0.0f';
    case 'double':
      return '0.0';
    default:
      return '';
  }
}

function getTypeMembVarCopyOp(type, membVar) {
  if (isIntType(type)) {
    return membVar;
  }
  switch (type) {
    case 'float':
    case 'double':
      return membVar;
    default:
      return membVar;
  }
}

function getTypeMembVarMoveOp(type, membVar) {
  if (isIntType(type)) {
    return membVar;
  }
  switch (type) {
    case 'float':
    case 'double':
      return membVar;
    default:
      return `std::move(${membVar})`;
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
    propertyJson.getterNoexcept = isPrimitiveType(propertyJson.type);
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
    propertyJson.setterNoexcept = isPrimitiveType(propertyJson.type);
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
  getName,
  getStyleName,
  getLangName,
  defaultClassNameLangToTransformationMap,
  getLangTransformedName,
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
  getLangAliases,
  getHeaderGuardName,
  defaultPropNameLangToTransformationMap,
  getPropertyName,
  getFloatType,
  getDoubleType,
  getIntType,
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
  isPrimitiveType,
  getTypeImplicitDefaultValue,
  getTypeMembVarCopyOp,
  getTypeMembVarMoveOp,
  defaultPropGetterNameLangToTransformationMap,
  getPropertyGetterName,
  defaultPropSetterNameLangToTransformationMap,
  getPropertySetterName,
  defaultPropSetterArgNameLangToTransformationMap,
  getPropertySetterArgName,
  getProperties,
  convertNameStyle,
  defaultClassNameLangToStyleMap,
  fillName,
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
  fillMoveConstructor
};
