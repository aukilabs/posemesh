const fs = require('fs');
const path = require('path');
const util = require('./util');

function generateHeader(interfaceName, interfaceJson) {
  const name = util.getLangClassName(interfaceJson, util.ObjC);
  const nameSwift = util.getLangClassName(interfaceJson, util.Swift);
  const copyable = util.getClassCopyable(interfaceJson);
  const copyableExt = copyable ? '<NSCopying>' : '';
  const static = util.getClassStatic(interfaceJson);
  const nativeGetterName = `native${nameSwift}`;

  let importsFirst = new Set(['#import <Foundation/Foundation.h>']), importsSecond = new Set(['#import "API.h"']);
  let includesFirst = new Set([]), includesSecond = new Set([]);

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += '%INCLUDES%\n';
  code += `NS_SWIFT_NAME(${nameSwift}) PSM_API @interface ${name} : NSObject${copyableExt}\n`;
  if (!static) {
    code += '\n';
    code += '#if defined(POSEMESH_BUILD)\n';
    code += `- (void*)${nativeGetterName};\n`;
    code += '#endif\n';
  }
  code += '\n';
  code += '@end\n'

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

function generateSource(interfaceName, interfaceJson) {
  let importsFirst = new Set([]), importsSecond = new Set([]);
  let includesFirst = new Set([]), includesSecond = new Set([]);

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += '%INCLUDES%\n';

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

function generateInterfaceObjC(interfaceName, interfaceJson) {
  const headerFilePath = path.resolve(__dirname, '..', 'platform', 'Apple', 'include', 'Posemesh', `${interfaceName}.h`);
  const sourceFilePath = path.resolve(__dirname, '..', 'platform', 'Apple', 'src', `${interfaceName}.mm`);

  let headerCode = generateHeader(interfaceName, interfaceJson);
  let sourceCode = generateSource(interfaceName, interfaceJson);

  fs.writeFileSync(headerFilePath, headerCode, 'utf8');
  fs.writeFileSync(sourceFilePath, sourceCode, 'utf8');
}

module.exports = generateInterfaceObjC;
