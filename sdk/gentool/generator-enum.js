const path = require('path');
const util = require('./util');

function generateEnumC(enums, enumName, enumJson) {
  const name = util.getLangEnumName(enumJson, util.C);
  const nameWithoutESuffix = name.substring(0, name.length - 2);
  const type = enumJson['type'];
  const isFlagType = type === 'flag';
  const constants = enumJson['constants'];
  const aliases = enumJson['aliases'];
  const headerGuardName = util.getHeaderGuardName(enumJson);
  const headerGuard = `__POSEMESH_C_${headerGuardName}_H__`;

  let headerCode = `/* This code is automatically generated from ${enumName}.json enum. Do not modify it manually as it will be overwritten! */\n`;
  headerCode += `\n`;
  headerCode += `#ifndef ${headerGuard}\n`;
  headerCode += `#define ${headerGuard}\n`;
  headerCode += `\n`;
  headerCode += `#include <stdint.h>\n`;
  headerCode += `\n`;
  headerCode += `enum ${nameWithoutESuffix} {`;
  let firstConstant = true;
  for (const constantJson of constants) {
    if (firstConstant) {
      firstConstant = false;
    } else {
      headerCode += `,`;
    }
    headerCode += `\n`;
    const constantName = util.getLangEnumConstantName(constantJson, util.C);
    headerCode += `    PSM_${util.getStyleName('name', enumJson, util.UPPER_CASE)}_${constantName} = ${constantJson.value}${isFlagType ? 'u' : ''}`;
  }
  if (constants.length > 0) {
    headerCode += `\n};\n`;
  } else {
    headerCode += ` _ };\n`;
  }
  headerCode += `typedef enum ${nameWithoutESuffix} ${name};\n`;
  for (const aliasJson of aliases) {
    const alias = util.getLangEnumName(aliasJson, util.C);
    const aliasWithoutESuffix = alias.substring(0, alias.length - 2);
    headerCode += `\n`;
    headerCode += `enum ${aliasWithoutESuffix} {`;
    let firstAliasConstant = true;
    for (const constantJson of constants) {
      if (firstAliasConstant) {
        firstAliasConstant = false;
      } else {
        headerCode += `,`;
      }
      headerCode += `\n`;
      const constantName = util.getLangEnumConstantName(constantJson, util.C);
      headerCode += `    PSM_${util.getStyleName('name', aliasJson, util.UPPER_CASE)}_${constantName} = PSM_${util.getStyleName('name', enumJson, util.UPPER_CASE)}_${constantName}`;
    }
    if (constants.length > 0) {
      headerCode += `\n};\n`;
    } else {
      headerCode += ` _ };\n`;
    }
    headerCode += `typedef enum ${aliasWithoutESuffix} ${alias};\n`;
  }
  headerCode += `\n`;
  headerCode += `#endif /* ${headerGuard} */\n`;

  const headerFilePath = path.resolve(__dirname, '..', 'include', 'Posemesh', 'C', `${enumName}.h`);
  util.writeFileContentIfDifferent(headerFilePath, headerCode);
}

function generateEnumCXX(enums, enumName, enumJson) {
  const name = util.getLangEnumName(enumJson, util.CXX);
  const type = enumJson['type'];
  const isFlagType = type === 'flag';
  const constants = enumJson['constants'];
  const aliases = util.getLangAliases(enumJson, util.CXX, util.defaultEnumNameLangToTransformationMap);
  const headerGuardName = util.getHeaderGuardName(enumJson);
  const headerGuard = `__POSEMESH_${headerGuardName}_HPP__`;

  let headerCode = `/* This code is automatically generated from ${enumName}.json enum. Do not modify it manually as it will be overwritten! */\n`;
  headerCode += `\n`;
  headerCode += `#ifndef ${headerGuard}\n`;
  headerCode += `#define ${headerGuard}\n`;
  headerCode += `\n`;
  headerCode += `#include <cstdint>\n`;
  headerCode += `\n`;
  headerCode += `namespace psm {\n`;
  headerCode += `\n`;
  headerCode += `enum class ${name} : ${isFlagType ? 'std::uint32_t' : 'std::int32_t'} {`;
  let firstConstant = true;
  for (const constantJson of constants) {
    if (firstConstant) {
      firstConstant = false;
    } else {
      headerCode += `,`;
    }
    headerCode += `\n`;
    const constantName = util.getLangEnumConstantName(constantJson, util.CXX);
    headerCode += `    ${constantName} = ${constantJson.value}${isFlagType ? 'u' : ''}`;
  }
  if (constants.length > 0) {
    headerCode += `\n};\n`;
  } else {
    headerCode += ` _ };\n`;
  }
  for (const alias of aliases) {
    headerCode += `using ${alias} = ${name};\n`;
  }
  headerCode += `\n`;
  headerCode += `}\n`;
  headerCode += `\n`;
  headerCode += `#endif // ${headerGuard}\n`;

  const headerFilePath = path.resolve(__dirname, '..', 'include', 'Posemesh', `${enumName}.hpp`);
  util.writeFileContentIfDifferent(headerFilePath, headerCode);
}

function generateEnumJS(enums, enumName, enumJson) {
  const name = util.getLangEnumName(enumJson, util.JS);
  const constants = enumJson['constants'];
  const aliases = util.getLangAliases(enumJson, util.JS, util.defaultEnumNameLangToTransformationMap);

  let sourceCode = `/* This code is automatically generated from ${enumName}.json enum. Do not modify it manually as it will be overwritten! */\n`;
  sourceCode += `\n`;
  sourceCode += `posemeshModule.${name} = null;\n`;
  for (const alias of aliases) {
    sourceCode += `posemeshModule.${alias} = null;\n`;
  }
  sourceCode += `\n`;
  sourceCode += `__internalPosemeshAPI.builderFunctions.push(function() {\n`;
  sourceCode += `    posemeshModule.${name} = Object.freeze({`;
  let firstConstant = true;
  for (const constantJson of constants) {
    if (firstConstant) {
      firstConstant = false;
    } else {
      sourceCode += `,`;
    }
    sourceCode += `\n`;
    const constantName = util.getLangEnumConstantName(constantJson, util.JS);
    sourceCode += `        ${constantName}: ${constantJson.value}`;
  }
  if (constants.length > 0) {
    sourceCode += `\n    });\n`;
  } else {
    sourceCode += `});\n`;
  }
  for (const alias of aliases) {
    sourceCode += `    posemeshModule.${alias} = posemeshModule.${name};\n`;
  }
  sourceCode += `});\n`;

  const sourceFilePath = path.resolve(__dirname, '..', 'platform', 'Web', `${enumName}.js`);
  util.writeFileContentIfDifferent(sourceFilePath, sourceCode);
}

function generateEnumTransformTsDef(enums, enumName, enumJson) {
  let sourceCode = `/* This code is automatically generated from ${enumName}.json enum. Do not modify it manually as it will be overwritten! */\n`;
  // TODO: needs to be implemented

  const sourceFilePath = path.resolve(__dirname, '..', 'platform', 'Web', `transform-typescript-definition-${enumName}.js`);
  util.writeFileContentIfDifferent(sourceFilePath, sourceCode);
}

function generateEnumObjC(enums, enumName, enumJson) {
  const name = util.getLangEnumName(enumJson, util.ObjC);
  const nameSwift = util.getLangEnumName(enumJson, util.Swift);
  const type = enumJson['type'];
  const isFlagType = type === 'flag';
  const constants = enumJson['constants'];
  const aliases = enumJson['aliases'];

  let headerCode = `/* This code is automatically generated from ${enumName}.json enum. Do not modify it manually as it will be overwritten! */\n`;
  headerCode += `\n`;
  headerCode += `#import <Foundation/Foundation.h>\n`;
  headerCode += `\n`;
  headerCode += `typedef NS_ENUM(${isFlagType ? 'NSUInteger' : 'NSInteger'}, ${name}) {`;
  let firstConstant = true;
  for (const constantJson of constants) {
    if (firstConstant) {
      firstConstant = false;
    } else {
      headerCode += `,`;
    }
    headerCode += `\n`;
    const constantName = util.getLangEnumConstantName(constantJson, util.ObjC);
    headerCode += `    ${name}${constantName} = ${constantJson.value}${isFlagType ? 'u' : ''}`;
  }
  if (constants.length > 0) {
    headerCode += `\n} NS_SWIFT_NAME(${nameSwift});\n`;
  } else {
    headerCode += ` _ } NS_SWIFT_NAME(${nameSwift});\n`;
  }
  if (aliases.length > 0) {
    headerCode += `\n`;
    headerCode += `#if defined(__swift__)\n`;
    for (const aliasJson of aliases) {
      const alias = util.getLangEnumName(aliasJson, util.ObjC);
      const aliasSwift = util.getLangEnumName(aliasJson, util.Swift);
      headerCode += `typedef ${name} __${alias} NS_SWIFT_NAME(${aliasSwift});\n`;
    }
    headerCode += `#else\n`;
    for (const aliasJson of aliases) {
      const alias = util.getLangEnumName(aliasJson, util.ObjC);
      headerCode += `typedef NS_ENUM(${isFlagType ? 'NSUInteger' : 'NSInteger'}, ${alias}) {`;
      let firstAliasConstant = true;
      for (const constantJson of constants) {
        if (firstAliasConstant) {
          firstAliasConstant = false;
        } else {
          headerCode += `,`;
        }
        headerCode += `\n`;
        const constantName = util.getLangEnumConstantName(constantJson, util.ObjC);
        headerCode += `    ${alias}${constantName} = ${name}${constantName}`;
      }
      if (constants.length > 0) {
        headerCode += `\n};\n`;
      } else {
        headerCode += ` _ };\n`;
      }
    }
    headerCode += `#endif\n`;
  }

  const headerFilePath = path.resolve(__dirname, '..', 'platform', 'Apple', 'include', 'Posemesh', `${enumName}.h`);
  util.writeFileContentIfDifferent(headerFilePath, headerCode);
}

function generateEnum(enums, enumName, enumJson) {
  generateEnumC(enums, enumName, enumJson);
  generateEnumCXX(enums, enumName, enumJson);
  generateEnumJS(enums, enumName, enumJson);
  generateEnumTransformTsDef(enums, enumName, enumJson);
  generateEnumObjC(enums, enumName, enumJson);
}

module.exports = generateEnum;
