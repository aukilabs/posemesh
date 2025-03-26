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
        prop += `            return __${util.getPropertyGetterName(propertyJson, util.ObjC)}()\n`;
        prop += `        }\n`;
      }
      if (hasPublicSetter) {
        prop += `        set {\n`;
        prop += `            __${util.getPropertySetterName(propertyJson, util.ObjC)}(newValue)\n`;
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
