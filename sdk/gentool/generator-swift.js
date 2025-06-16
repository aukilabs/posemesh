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
  for (const methodJson of interfaceJson.methods) {
    let methodNameSwift = util.getLangName('name', methodJson, util.Swift);
    let methodNameObjC = util.getLangName('name', methodJson, util.ObjC);
    const methodReturnTypeSwift = methodJson.returnType.length > 0 ? util.getPropertyTypeForGetter(enums, interfaces, { "type": methodJson.returnType }, util.Swift) : 'Void';
    const methodReturnTypeObjC = methodJson.returnType.length > 0 ? util.getPropertyTypeForGetter(enums, interfaces, { "type": methodJson.returnType }, util.ObjC) : 'void';
    const methodStatic = methodJson.static;
    const methodVisibility = methodJson.visibility;
    if (methodVisibility !== util.Visibility.public) {
      continue;
    }
    let argsSwift = '';
    let argsObjC = '';
    for (const parameterJson of methodJson.parameters) {
      const parameterName = util.getLangName('name', parameterJson, util.ObjC);
      const parameterNameFixed = parameterName.charAt(0).toUpperCase() + parameterName.slice(1);
      const parameterType = util.getPropertyTypeForSetter(enums, interfaces, parameterJson, util.Swift);
      const parameterObjCNamePrefix = parameterJson['objectiveC.namePrefix'];
      const parameterObjCNamePrefixFixed = parameterObjCNamePrefix.charAt(0).toUpperCase() + parameterObjCNamePrefix.slice(1);
      const parameterSwiftNamePrefix = parameterJson['swift.namePrefix'];
      const parameterSwiftNamePrefixFixed = parameterSwiftNamePrefix.charAt(0).toUpperCase() + parameterSwiftNamePrefix.slice(1);
      let parameterEval = parameterName;
      if (util.isArrayOfAnyType(parameterJson.type)) {
        const innerType = parameterJson.type.split(':').slice(1).join(':');
        if (innerType in enums) {
          parameterEval = `${parameterName}.map { NSNumber(value: $0.rawValue) }`;
        } else if (innerType in interfaces) {
          if (util.isArrayPtrType(parameterJson.type) || util.isArrayPtrRefType(parameterJson.type)) {
            parameterEval = `${parameterName}.map { $0 ?? NSNull() }`;
          }
        } else if (util.isIntType(innerType) || innerType === 'float' || innerType === 'double' || innerType === 'boolean') {
          parameterEval = `${parameterName}.map { NSNumber(value: $0) }`;
        }
      }
      if (argsSwift.length > 0) {
        argsSwift += ', ';
        if (parameterSwiftNamePrefix === '') {
          argsSwift += `${parameterName} ${parameterName}: ${parameterType}`;
        } else if (parameterSwiftNamePrefix === '-') {
          argsSwift += `${parameterName}: ${parameterType}`;
        } else {
          argsSwift += `${parameterSwiftNamePrefix}${parameterNameFixed} ${parameterName}: ${parameterType}`;
        }
      } else {
        if (parameterSwiftNamePrefix === '') {
          argsSwift += `${parameterName} ${parameterName}: ${parameterType}`;
        } else if (parameterSwiftNamePrefix === '-') {
          argsSwift += `${parameterName}: ${parameterType}`;
        } else {
          argsSwift += `${parameterSwiftNamePrefix}${parameterNameFixed} ${parameterName}: ${parameterType}`;
        }
      }
      if (argsObjC.length > 0) {
        argsObjC += ', ';
        if (parameterObjCNamePrefix === '') {
          argsObjC += `${parameterName}: ${parameterEval}`;
        } else if (parameterObjCNamePrefix === '-') {
          argsObjC += `${parameterEval}`;
        } else {
          argsObjC += `${parameterObjCNamePrefix}${parameterNameFixed}: ${parameterEval}`;
        }
      } else {
        if (parameterObjCNamePrefix === '') {
          methodNameObjC += `${parameterObjCNamePrefixFixed}`;
          argsObjC += `${parameterEval}`;
        } else if (parameterObjCNamePrefix === '-') {
          argsObjC += `${parameterEval}`;
        } else {
          argsObjC += `${parameterObjCNamePrefix}${parameterNameFixed}: ${parameterEval}`;
        }
      }
    }
    let postMap = '';
    if (util.isArrayOfAnyType(methodJson.returnType)) {
      const innerType = methodJson.returnType.split(':').slice(1).join(':');
      if (innerType in enums) {
        if (enums[innerType].type === 'flag') {
          postMap += `.map { ${util.getLangEnumName(enums[innerType], util.Swift)}(rawValue: $0.uintValue)! }`;
        } else {
          postMap += `.map { ${util.getLangEnumName(enums[innerType], util.Swift)}(rawValue: $0.intValue)! }`;
        }
      } else if (innerType in interfaces) {
        if (util.isArrayPtrType(methodJson.returnType) || util.isArrayPtrRefType(methodJson.returnType)) {
          postMap += `.map { $0 as? ${util.getLangClassName(interfaces[innerType], util.Swift)} }`;
        }
      } else if (innerType === 'int8') {
        postMap += `.map { $0.int8Value }`;
      } else if (innerType === 'int16') {
        postMap += `.map { $0.int16Value }`;
      } else if (innerType === 'int32') {
        postMap += `.map { $0.int32Value }`;
      } else if (innerType === 'int64') {
        postMap += `.map { $0.int64Value }`;
      } else if (innerType === 'uint8') {
        postMap += `.map { $0.uint8Value }`;
      } else if (innerType === 'uint16') {
        postMap += `.map { $0.uint16Value }`;
      } else if (innerType === 'uint32') {
        postMap += `.map { $0.uint32Value }`;
      } else if (innerType === 'uint64') {
        postMap += `.map { $0.uint64Value }`;
      } else if (innerType === 'float') {
        postMap += `.map { $0.floatValue }`;
      } else if (innerType === 'double') {
        postMap += `.map { $0.doubleValue }`;
      } else if (innerType === 'boolean') {
        postMap += `.map { $0.boolValue }`;
      }
    }
    let prop = '';
    prop += `    public${methodStatic ? ' static' : ''} func ${methodNameSwift}(${argsSwift}) -> ${methodReturnTypeSwift} {\n`;
    prop += `        return __${methodNameObjC}(${argsObjC})${postMap};\n`;
    prop += `    }\n`;
    if (methodStatic) {
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
