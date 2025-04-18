const path = require('path');
const util = require('./util');

function generateSource(enums, interfaces, interfaceName, interfaceJson) {
  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += `\n`;
  code += `extension ${util.getLangClassName(interfaceJson, util.Swift)} {\n`;
  let codeMemberProps = '', codeStaticProps = '';
  for (const propertyJson of util.getProperties(interfaceJson)) {
    const hasPublicGetter = propertyJson.hasGetter && util.getPropertyGetterVisibility(propertyJson) === util.Visibility.public;
    const hasPublicSetter = propertyJson.hasSetter && util.getPropertySetterVisibility(propertyJson) === util.Visibility.public;
    if (hasPublicGetter || hasPublicSetter) {
      let prop = '';
      prop += `    public${util.getPropertyStatic(propertyJson) ? ' static' : ''} var ${util.getPropertyName(propertyJson, util.Swift)}: ${util.getPropertyType(enums, interfaces, propertyJson, util.Swift)} {\n`;
      if (hasPublicGetter) {
        prop += `        get {\n`;
        if (util.isArrayOfAnyType(propertyJson.type)) {
          const propTypeRaw = propertyJson.type;
          const underlyingArrayTypeRaw = propTypeRaw.split(':').slice(1).join(':');
          if (underlyingArrayTypeRaw === 'float') {
            prop += `            return __${util.getPropertyGetterName(propertyJson, util.ObjC)}().map { $0.floatValue }\n`;
          } else if (underlyingArrayTypeRaw === 'double') {
            prop += `            return __${util.getPropertyGetterName(propertyJson, util.ObjC)}().map { $0.doubleValue }\n`;
          } else if (underlyingArrayTypeRaw === 'int8') {
            prop += `            return __${util.getPropertyGetterName(propertyJson, util.ObjC)}().map { $0.int8Value }\n`;
          } else if (underlyingArrayTypeRaw === 'int16') {
            prop += `            return __${util.getPropertyGetterName(propertyJson, util.ObjC)}().map { $0.int16Value }\n`;
          } else if (underlyingArrayTypeRaw === 'int32') {
            prop += `            return __${util.getPropertyGetterName(propertyJson, util.ObjC)}().map { $0.int32Value }\n`;
          } else if (underlyingArrayTypeRaw === 'int64') {
            prop += `            return __${util.getPropertyGetterName(propertyJson, util.ObjC)}().map { $0.int64Value }\n`;
          } else if (underlyingArrayTypeRaw === 'uint8') {
            prop += `            return __${util.getPropertyGetterName(propertyJson, util.ObjC)}().map { $0.uint8Value }\n`;
          } else if (underlyingArrayTypeRaw === 'uint16') {
            prop += `            return __${util.getPropertyGetterName(propertyJson, util.ObjC)}().map { $0.uint16Value }\n`;
          } else if (underlyingArrayTypeRaw === 'uint32') {
            prop += `            return __${util.getPropertyGetterName(propertyJson, util.ObjC)}().map { $0.uint32Value }\n`;
          } else if (underlyingArrayTypeRaw === 'uint64') {
            prop += `            return __${util.getPropertyGetterName(propertyJson, util.ObjC)}().map { $0.uint64Value }\n`;
          } else if (underlyingArrayTypeRaw === 'boolean') {
            prop += `            return __${util.getPropertyGetterName(propertyJson, util.ObjC)}().map { $0.boolValue }\n`;
          } else if (underlyingArrayTypeRaw === 'string') {
            prop += `            return __${util.getPropertyGetterName(propertyJson, util.ObjC)}()\n`;
          } else if (typeof enums[underlyingArrayTypeRaw] !== 'undefined') {
            if (enums[underlyingArrayTypeRaw].type === 'flag') {
              prop += `            return __${util.getPropertyGetterName(propertyJson, util.ObjC)}().map { ${util.getLangEnumName(enums[underlyingArrayTypeRaw], util.Swift)}(rawValue: $0.uintValue)! }\n`;
            } else {
              prop += `            return __${util.getPropertyGetterName(propertyJson, util.ObjC)}().map { ${util.getLangEnumName(enums[underlyingArrayTypeRaw], util.Swift)}(rawValue: $0.intValue)! }\n`;
            }
          } else if (typeof interfaces[underlyingArrayTypeRaw] !== 'undefined') {
            if (util.isArrayType(propTypeRaw) || util.isArrayRefType(propTypeRaw) || util.isArrayMixType(propTypeRaw)) {
              prop += `            return __${util.getPropertyGetterName(propertyJson, util.ObjC)}()\n`;
            } else {
              prop += `            return __${util.getPropertyGetterName(propertyJson, util.ObjC)}().map { $0 as? ${util.getLangClassName(interfaces[underlyingArrayTypeRaw], util.Swift)} }\n`;
            }
          } else {
            throw new Error(`Unhandled type: ${propTypeRaw}`);
          }
        } else {
          prop += `            return __${util.getPropertyGetterName(propertyJson, util.ObjC)}()\n`;
        }
        prop += `        }\n`;
      }
      if (hasPublicSetter) {
        prop += `        set {\n`;
        if (util.isArrayOfAnyType(propertyJson.type)) {
          const propTypeRaw = propertyJson.type;
          const underlyingArrayTypeRaw = propTypeRaw.split(':').slice(1).join(':');
          if (underlyingArrayTypeRaw === 'float' || underlyingArrayTypeRaw === 'double' || util.isIntType(underlyingArrayTypeRaw) || underlyingArrayTypeRaw === 'boolean') {
            prop += `            __${util.getPropertySetterName(propertyJson, util.ObjC)}(newValue.map { NSNumber(value: $0) })\n`;
          } else if (underlyingArrayTypeRaw === 'string') {
            prop += `            __${util.getPropertySetterName(propertyJson, util.ObjC)}(newValue)\n`;
          } else if (typeof enums[underlyingArrayTypeRaw] !== 'undefined') {
            prop += `            __${util.getPropertySetterName(propertyJson, util.ObjC)}(newValue.map { NSNumber(value: $0.rawValue) })\n`;
          } else if (typeof interfaces[underlyingArrayTypeRaw] !== 'undefined') {
            if (util.isArrayType(propTypeRaw) || util.isArrayRefType(propTypeRaw) || util.isArrayMixType(propTypeRaw)) {
              prop += `            __${util.getPropertySetterName(propertyJson, util.ObjC)}(newValue)\n`;
            } else {
              prop += `            __${util.getPropertySetterName(propertyJson, util.ObjC)}(newValue.map { $0 ?? NSNull() })\n`;
            }
          } else {
            throw new Error(`Unhandled type: ${propTypeRaw}`);
          }
        } else {
          prop += `            __${util.getPropertySetterName(propertyJson, util.ObjC)}(newValue)\n`;
        }
        prop += `        }\n`;
      }
      prop += `    }\n`;
      if (util.getPropertyStatic(propertyJson)) {
        if (codeStaticProps.length > 0) {
          codeStaticProps += `\n`;
        }
        codeStaticProps += prop;
      } else {
        if (codeMemberProps.length > 0) {
          codeMemberProps += `\n`;
        }
        codeMemberProps += prop;
      }
    }
  }
  code += codeMemberProps;
  if (codeMemberProps.length > 0 && codeStaticProps.length > 0) {
    code += `\n`;
  }
  code += codeStaticProps;
  code += `}\n`;
  return code;
}

function generateInterfaceSwift(enums, interfaces, interfaceName, interfaceJson) {
  const sourceFilePath = path.resolve(__dirname, '..', 'platform', 'Apple', 'src', `${interfaceName}.swift`);

  let sourceCode = generateSource(enums, interfaces, interfaceName, interfaceJson);

  util.writeFileContentIfDifferent(sourceFilePath, sourceCode);
}

module.exports = generateInterfaceSwift;
