const path = require('path');
const util = require('./util');

function doesArrayTypeNeedFree(interfaces, type) {
  if (!util.isArrayOfAnyType(type)) {
    return false;
  }
  if (util.isArrayType(type) || util.isArrayPtrType(type)) {
    return true;
  }
  const name = type.split(':').slice(1).join(':');
  return name === 'boolean' || name === 'string' || typeof interfaces[name] !== 'undefined';
}

function doesArrayGetterFreeFuncHaveOptionToDestroyContainedClasses(interfaces, type) {
  if (!util.isArrayType(type) && !util.isArrayPtrType(type)) {
    return false;
  }
  const name = type.split(':').slice(1).join(':');
  return typeof interfaces[name] !== 'undefined';
}

function arrayGetterIncludes(enums, interfaces, propTypeRaw, includesFirst, includesSecond) {
  const underlyingArrayTypeRaw = propTypeRaw.split(':').slice(1).join(':');
  if (doesArrayTypeNeedFree(interfaces, propTypeRaw)) {
    includesFirst.add('#include <new>');
    if (underlyingArrayTypeRaw === 'string') {
      if (util.isArrayType(propTypeRaw)) {
        includesFirst.add('#include <cstddef>');
        includesFirst.add('#include <cstring>');
      }
    } else if (typeof enums[underlyingArrayTypeRaw] !== 'undefined') {
      includesFirst.add('#include <type_traits>');
      includesFirst.add('#include <cstring>');
    } else if (typeof interfaces[underlyingArrayTypeRaw] !== 'undefined') {
      includesFirst.add('#include <type_traits>');
      if (util.isArrayType(propTypeRaw) || util.isArrayPtrType(propTypeRaw)) {
        includesFirst.add('#include <utility>');
        if (util.isArrayPtrType(propTypeRaw)) {
          includesFirst.add('#include <memory>');
        }
        includesFirst.add('#include <cstddef>');
      }
    } else {
      includesFirst.add('#include <type_traits>');
      includesFirst.add('#include <cstring>');
    }
  } else if (typeof enums[underlyingArrayTypeRaw] !== 'undefined') {
    includesFirst.add('#include <type_traits>');
  }
}

function arrayGetterCode(enums, interfaces, propertyJson, propTypeRaw, mainArgName, setterArgName, nameCxx, propStatic) {
  let getter = '';
  const underlyingArrayTypeRaw = propTypeRaw.split(':').slice(1).join(':');
  if (doesArrayTypeNeedFree(interfaces, propTypeRaw)) {
    const isConstRetType = (!util.isArrayType(propTypeRaw) && !util.isArrayPtrType(propTypeRaw)) || typeof interfaces[underlyingArrayTypeRaw] === 'undefined';
    const isRefRetType = !util.isArrayType(propTypeRaw) && !util.isArrayPtrType(propTypeRaw);
    getter += `    ${isConstRetType ? 'const ' : ''}auto${isRefRetType ? '&' : ''} ${setterArgName} = ${propStatic ? `psm::${nameCxx}::` : `${mainArgName}->`}${util.getPropertyGetterName(propertyJson, util.CXX)}();\n`;
    if (underlyingArrayTypeRaw === 'boolean') {
      getter += `    auto* getter_result = new (std::nothrow) std::uint8_t[${setterArgName}.size()];\n`;
    } else if (underlyingArrayTypeRaw === 'string') {
      if (util.isArrayType(propTypeRaw)) {
        getter += `    std::size_t getter_result_size = (${setterArgName}.size() + 1) * sizeof(char*);\n`;
        getter += `    for (const std::string& ${setterArgName}_element : ${setterArgName}) {\n`;
        getter += `        getter_result_size += ${setterArgName}_element.size() + 1;\n`;
        getter += `    }\n`;
        getter += `    auto* getter_result = new (std::nothrow) char[getter_result_size];\n`;
      } else {
        getter += `    auto** getter_result = new (std::nothrow) char*[${setterArgName}.size() + 1];\n`;
      }
    } else if (typeof enums[underlyingArrayTypeRaw] !== 'undefined') {
      getter += `    auto* getter_result = new (std::nothrow) ${util.getLangEnumName(enums[underlyingArrayTypeRaw], util.C)}[${setterArgName}.size()];\n`;
    } else if (typeof interfaces[underlyingArrayTypeRaw] !== 'undefined') {
      if (util.isArrayType(propTypeRaw) || util.isArrayPtrType(propTypeRaw)) {
        getter += `    auto** getter_result = new (std::nothrow) std::remove_reference_t<decltype(${setterArgName})>::value_type*[${setterArgName}.size() + 2];\n`;
      } else {
        getter += `    auto** getter_result = new (std::nothrow) std::remove_reference_t<decltype(${setterArgName})>::value_type*[${setterArgName}.size() + 1];\n`;
      }
    } else {
      getter += `    auto* getter_result = new (std::nothrow) std::remove_reference_t<decltype(${setterArgName})>::value_type[${setterArgName}.size()];\n`;
    }
    getter += `    if (!getter_result) {\n`;
    getter += `        if (out_length) {\n`;
    getter += `            *out_length = 0;\n`;
    getter += `        }\n`;
    getter += `        return nullptr;\n`;
    getter += `    }\n`;
    if (underlyingArrayTypeRaw === 'boolean') {
      getter += `    auto* out_getter_result = getter_result;\n`;
      getter += `    for (bool ${setterArgName}_element : ${setterArgName}) {\n`;
      getter += `        *out_getter_result++ = static_cast<std::uint8_t>(${setterArgName}_element);\n`;
      getter += `    }\n`;
    } else if (underlyingArrayTypeRaw === 'string') {
      if (util.isArrayType(propTypeRaw)) {
        getter += `    auto** out_getter_result_strings = reinterpret_cast<char**>(getter_result);\n`;
        getter += `    auto* out_getter_result_chars = getter_result + (${setterArgName}.size() + 1) * sizeof(char*);\n`;
        getter += `    for (const std::string& ${setterArgName}_element : ${setterArgName}) {\n`;
        getter += `        *out_getter_result_strings++ = out_getter_result_chars;\n`;
        getter += `        std::memcpy(out_getter_result_chars, ${setterArgName}_element.c_str(), ${setterArgName}_element.size() + 1);\n`;
        getter += `        out_getter_result_chars += ${setterArgName}_element.size() + 1;\n`;
        getter += `    }\n`;
        getter += `    *out_getter_result_strings = nullptr;\n`;
      } else {
        getter += `    const auto** out_getter_result = const_cast<const char**>(getter_result);\n`;
        getter += `    for (const std::string& ${setterArgName}_element : ${setterArgName}) {\n`;
        getter += `        *out_getter_result++ = ${setterArgName}_element.c_str();\n`;
        getter += `    }\n`;
        getter += `    *out_getter_result = nullptr;\n`;
      }
    } else if (typeof enums[underlyingArrayTypeRaw] !== 'undefined') {
      getter += `    static_assert(sizeof(${util.getLangEnumName(enums[underlyingArrayTypeRaw], util.C)}) == sizeof(std::remove_reference_t<decltype(${setterArgName})>::value_type));\n`;
      getter += `    std::memcpy(getter_result, ${setterArgName}.data(), ${setterArgName}.size() * sizeof(std::remove_reference_t<decltype(${setterArgName})>::value_type));\n`;
    } else if (typeof interfaces[underlyingArrayTypeRaw] !== 'undefined') {
      const wrapInSPtr = util.isArrayPtrType(propTypeRaw) || util.isArrayPtrRefType(propTypeRaw) || util.isArrayPtrMixType(propTypeRaw);
      getter += `    const auto** out_getter_result = const_cast<const ${wrapInSPtr ? 'std::shared_ptr<' : ''}psm::${util.getLangClassName(interfaces[underlyingArrayTypeRaw], util.CXX)}${wrapInSPtr ? '>' : ''}**>(getter_result);\n`;
      if (util.isArrayType(propTypeRaw) || util.isArrayPtrType(propTypeRaw)) {
        getter += `    *out_getter_result++ = reinterpret_cast<std::remove_reference_t<decltype(${setterArgName})>::value_type*>(${setterArgName}.size());\n`
      }
      getter += `    for (auto& ${setterArgName}_element : ${setterArgName}) {\n`;
      if (util.isArrayType(propTypeRaw)) {
        getter += `        *out_getter_result++ = new (std::nothrow) psm::${util.getLangClassName(interfaces[underlyingArrayTypeRaw], util.CXX)}(std::move(${setterArgName}_element));\n`;
      } else if (util.isArrayPtrType(propTypeRaw)) {
        getter += `        *out_getter_result++ = ${setterArgName}_element ? new (std::nothrow) std::shared_ptr<psm::${util.getLangClassName(interfaces[underlyingArrayTypeRaw], util.CXX)}>(std::move(${setterArgName}_element)) : nullptr;\n`;
      } else if (util.isArrayPtrRefType(propTypeRaw) || util.isArrayPtrMixType(propTypeRaw)) {
        getter += `        *out_getter_result++ = ${setterArgName}_element ? &${setterArgName}_element : nullptr;\n`;
      } else {
        getter += `        *out_getter_result++ = &${setterArgName}_element;\n`;
      }
      getter += `    }\n`;
      getter += `    *out_getter_result = nullptr;\n`;
    } else {
      getter += `    std::memcpy(getter_result, ${setterArgName}.data(), ${setterArgName}.size() * sizeof(std::remove_reference_t<decltype(${setterArgName})>::value_type));\n`;
    }
    getter += `    if (out_length) {\n`;
    getter += `        *out_length = static_cast<uint64_t>(${setterArgName}.size());\n`;
    getter += `    }\n`;
    if (underlyingArrayTypeRaw === 'string') {
      if (util.isArrayType(propTypeRaw)) {
        getter += `    return const_cast<const char**>(reinterpret_cast<char**>(getter_result));\n`;
      } else {
        getter += `    return const_cast<const char**>(getter_result);\n`;
      }
    } else if (typeof interfaces[underlyingArrayTypeRaw] !== 'undefined') {
      if (util.isArrayType(propTypeRaw) || util.isArrayPtrType(propTypeRaw)) {
        getter += `    return getter_result + 1;\n`;
      } else {
        const wrapInSPtr = util.isArrayPtrRefType(propTypeRaw) || util.isArrayPtrMixType(propTypeRaw);
        getter += `    return const_cast<const ${wrapInSPtr ? 'std::shared_ptr<' : ''}psm::${util.getLangClassName(interfaces[underlyingArrayTypeRaw], util.CXX)}${wrapInSPtr ? '>' : ''}**>(getter_result);\n`;
      }
    } else {
      getter += `    return getter_result;\n`;
    }
  } else {
    getter += `    const auto& ${setterArgName} = ${propStatic ? `psm::${nameCxx}::` : `${mainArgName}->`}${util.getPropertyGetterName(propertyJson, util.CXX)}();\n`;
    getter += `    if (out_length) {\n`;
    getter += `        *out_length = static_cast<uint64_t>(${setterArgName}.size());\n`;
    getter += `    }\n`;
    if (typeof enums[underlyingArrayTypeRaw] !== 'undefined') {
      getter += `    static_assert(sizeof(${util.getLangEnumName(enums[underlyingArrayTypeRaw], util.C)}) == sizeof(std::remove_reference_t<decltype(${setterArgName})>::value_type));\n`;
      getter += `    return reinterpret_cast<const ${util.getLangEnumName(enums[underlyingArrayTypeRaw], util.C)}*>(${setterArgName}.data());\n`;
    } else {
      getter += `    return ${setterArgName}.data();\n`;
    }
  }
  return getter;
}

function arraySetterIncludes(enums, interfaces, propTypeRaw, includesFirst, includesSecond) {
  includesFirst.add('#include <cassert>');
  const underlyingArrayTypeRaw = propTypeRaw.split(':').slice(1).join(':');
  if (underlyingArrayTypeRaw === 'string' || typeof interfaces[underlyingArrayTypeRaw] !== 'undefined') {
    includesFirst.add('#include <cstddef>');
  }
  if (underlyingArrayTypeRaw === 'boolean' || typeof interfaces[underlyingArrayTypeRaw] !== 'undefined') {
    if (typeof interfaces[underlyingArrayTypeRaw] !== 'undefined') {
      includesFirst.add('#include <type_traits>');
    }
    includesFirst.add('#include <algorithm>');
    includesFirst.add('#include <iterator>');
    if (util.isArrayType(propTypeRaw) || util.isArrayMixType(propTypeRaw) || util.isArrayPtrType(propTypeRaw) || util.isArrayPtrMixType(propTypeRaw)) {
      if (typeof interfaces[underlyingArrayTypeRaw] !== 'undefined') {
        includesFirst.add('#include <memory>');
      }
      includesFirst.add('#include <utility>');
    }
  }
}

function arraySetterCode(enums, interfaces, propertyJson, propTypeRaw, mainArgName, setterName, setterArgName, nameCxx, nameWithoutTSuffix, propStatic) {
  let setter = '';
  setter += `    if (!${setterArgName} && length > 0) {\n`;
  setter += `        assert(!"${nameWithoutTSuffix}_${setterName}(): ${setterArgName} is null and length is greater than zero");\n`;
  setter += `        return;\n`;
  setter += `    }\n`;
  const underlyingArrayTypeRaw = propTypeRaw.split(':').slice(1).join(':');
  if (underlyingArrayTypeRaw === 'string' || (typeof interfaces[underlyingArrayTypeRaw] !== 'undefined' && (util.isArrayType(propTypeRaw) || util.isArrayRefType(propTypeRaw) || util.isArrayMixType(propTypeRaw)))) {
    setter += `    for (std::size_t i = 0; i < length; ++i) {\n`;
    setter += `        if (!${setterArgName}[i]) {\n`;
    setter += `            assert(!"${nameWithoutTSuffix}_${setterName}(): ${setterArgName} contains at least one null element");\n`;
    setter += `            return;\n`;
    setter += `        }\n`;
    setter += `    }\n`;
  }
  const setterRoot = propStatic ? `psm::${nameCxx}::` : `${mainArgName}->`;
  if (underlyingArrayTypeRaw === 'boolean') {
    setter += `    std::vector<bool> transformed_vector;\n`;
    setter += `    transformed_vector.reserve(length);\n`;
    setter += `    std::transform(${setterArgName}, ${setterArgName} + length, std::back_inserter(transformed_vector), [](std::uint8_t ${setterArgName}_element) -> bool { return static_cast<bool>(${setterArgName}_element); });\n`;
    if (util.isArrayType(propTypeRaw) || util.isArrayMixType(propTypeRaw)) {
      setter += `    ${setterRoot}${util.getPropertySetterName(propertyJson, util.CXX)}(std::move(transformed_vector));\n`;
    } else {
      setter += `    ${setterRoot}${util.getPropertySetterName(propertyJson, util.CXX)}(transformed_vector);\n`;
    }
  } else if (typeof enums[underlyingArrayTypeRaw] !== 'undefined') {
    setter += `    static_assert(sizeof(${util.getLangEnumName(enums[underlyingArrayTypeRaw], util.C)}) == sizeof(psm::${util.getLangEnumName(enums[underlyingArrayTypeRaw], util.CXX)}));\n`;
    setter += `    ${setterRoot}${util.getPropertySetterName(propertyJson, util.CXX)}({ reinterpret_cast<const psm::${util.getLangEnumName(enums[underlyingArrayTypeRaw], util.CXX)}*>(${setterArgName}), reinterpret_cast<const psm::${util.getLangEnumName(enums[underlyingArrayTypeRaw], util.CXX)}*>(${setterArgName}) + length });\n`;
  } else if (typeof interfaces[underlyingArrayTypeRaw] !== 'undefined') {
    setter += `    std::vector<std::remove_const_t<std::remove_reference_t<decltype(**${setterArgName})>>> transformed_vector;\n`;
    setter += `    transformed_vector.reserve(length);\n`;
    if (util.isArrayType(propTypeRaw) || util.isArrayMixType(propTypeRaw) || util.isArrayPtrType(propTypeRaw) || util.isArrayPtrMixType(propTypeRaw)) {
      if (util.isArrayType(propTypeRaw) || util.isArrayMixType(propTypeRaw)) {
        setter += `    std::transform(${setterArgName}, ${setterArgName} + length, std::back_inserter(transformed_vector), [](std::remove_const_t<std::remove_reference_t<decltype(*${setterArgName})>> ${setterArgName}_element) -> decltype(transformed_vector)::value_type { std::unique_ptr<std::remove_const_t<std::remove_reference_t<decltype(**${setterArgName})>>> raii_wrapper(${setterArgName}_element); return std::move(*raii_wrapper); });\n`;
      } else {
        setter += `    std::transform(${setterArgName}, ${setterArgName} + length, std::back_inserter(transformed_vector), [](std::remove_const_t<std::remove_reference_t<decltype(*${setterArgName})>> ${setterArgName}_element) -> decltype(transformed_vector)::value_type { std::unique_ptr<std::remove_const_t<std::remove_reference_t<decltype(**${setterArgName})>>> raii_wrapper(${setterArgName}_element); return raii_wrapper ? decltype(transformed_vector)::value_type { std::move(*raii_wrapper) } : decltype(transformed_vector)::value_type {}; });\n`;
      }
      setter += `    ${setterRoot}${util.getPropertySetterName(propertyJson, util.CXX)}(std::move(transformed_vector));\n`;
    } else {
      if (util.isArrayRefType(propTypeRaw)) {
        setter += `    std::transform(${setterArgName}, ${setterArgName} + length, std::back_inserter(transformed_vector), [](std::remove_const_t<std::remove_reference_t<decltype(*${setterArgName})>> ${setterArgName}_element) -> decltype(transformed_vector)::value_type { return *${setterArgName}_element; });\n`;
      } else {
        setter += `    std::transform(${setterArgName}, ${setterArgName} + length, std::back_inserter(transformed_vector), [](std::remove_const_t<std::remove_reference_t<decltype(*${setterArgName})>> ${setterArgName}_element) -> decltype(transformed_vector)::value_type { return ${setterArgName}_element ? *${setterArgName}_element : decltype(transformed_vector)::value_type {}; });\n`;
      }
      setter += `    ${setterRoot}${util.getPropertySetterName(propertyJson, util.CXX)}(transformed_vector);\n`;
    }
  } else {
    setter += `    ${setterRoot}${util.getPropertySetterName(propertyJson, util.CXX)}({ ${setterArgName}, ${setterArgName} + length });\n`;
  }
  return setter;
}

function generateHeader(enums, interfaces, interfaceName, interfaceJson) {
  const name = util.getLangClassName(interfaceJson, util.C);
  const nameWithoutTSuffix = name.substring(0, name.length - 2);
  const nameRefWithoutTSuffix = `${nameWithoutTSuffix}_ref`;
  const nameRef = `${nameRefWithoutTSuffix}_t`;
  const nameCxx = util.getLangClassName(interfaceJson, util.CXX);
  const headerGuardName = util.getHeaderGuardName(interfaceJson);
  const headerGuard = `__POSEMESH_C_${headerGuardName}_H__`;
  const static = util.getClassStatic(interfaceJson);
  const copyable = util.getClassCopyable(interfaceJson);

  let includesFirst = new Set([]), includesSecond = new Set(['#include "API.h"']);

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += `\n`;
  code += `#ifndef ${headerGuard}\n`;
  code += `#define ${headerGuard}\n`;
  code += '%INCLUDES%\n';
  code += '#if defined(__cplusplus)\n';
  if (!static) {
    code += `#include <memory>\n`;
  }
  code += `namespace psm {\n`;
  code += `class ${nameCxx};\n`;
  code += `}\n`;
  code += `typedef psm::${nameCxx} ${name};\n`;
  if (!static) {
    code += `typedef std::shared_ptr<${name}> ${nameRef};\n`;
  }
  code += '#else\n';
  code += `typedef struct ${nameWithoutTSuffix} ${name};\n`;
  if (!static) {
    code += `typedef struct ${nameRefWithoutTSuffix} ${nameRef};\n`;
  }
  code += '#endif\n';
  for (const alias of util.getLangAliases(interfaceJson, util.C)) {
    code += `typedef ${name} ${alias};\n`;
  }
  if (!static) {
    for (const alias of util.getLangAliases(interfaceJson, util.C)) {
      code += `typedef ${nameRef} ${alias.substring(0, alias.length - 2)}_ref_t;\n`;
    }
  }

  code += `\n`;
  code += '#if defined(__cplusplus)\n';
  code += 'extern "C" {\n';
  code += '#endif\n';

  let publicCtors = '', publicOperators = '', publicMethods = '', publicFuncs = '';
  let funcAliases = [];

  const parameterlessConstructor = util.getClassParameterlessConstructor(interfaceJson);
  const pCtorDefinition = util.getConstructorDefinition(parameterlessConstructor);
  const pCtorVisibility = util.getConstructorVisibility(parameterlessConstructor);
  if (!static && pCtorDefinition !== util.ConstructorDefinition.deleted && pCtorVisibility === util.Visibility.public) {
    publicCtors += `${name}* PSM_API ${nameWithoutTSuffix}_create();\n`;
    funcAliases.push({
      name: `${nameWithoutTSuffix}_create`,
      args: []
    });
  }

  const copyConstructor = util.getClassCopyConstructor(interfaceJson);
  const cCtorDefinition = util.getConstructorDefinition(copyConstructor);
  const cCtorVisibility = util.getConstructorVisibility(copyConstructor);
  if (!static && copyable && cCtorDefinition !== util.ConstructorDefinition.deleted && cCtorVisibility === util.Visibility.public) {
    const mainArgName = util.getCopyOrMoveConstructorMainArgName(copyConstructor, util.C);
    publicCtors += `${name}* PSM_API ${nameWithoutTSuffix}_duplicate(const ${name}* ${mainArgName});\n`;
    funcAliases.push({
      name: `${nameWithoutTSuffix}_duplicate`,
      args: [mainArgName]
    });
  }

  if (!static) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    publicCtors += `void PSM_API ${nameWithoutTSuffix}_destroy(${name}* ${mainArgName});\n`;
    funcAliases.push({
      name: `${nameWithoutTSuffix}_destroy`,
      args: [mainArgName]
    });
  }

  const equalityOperator = interfaceJson.equalityOperator;
  if (equalityOperator.defined) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    publicOperators += `uint8_t PSM_API ${nameWithoutTSuffix}_equals(const ${name}* ${mainArgName}, const ${name}* other_${mainArgName});\n`;
    funcAliases.push({
      name: `${nameWithoutTSuffix}_equals`,
      args: [mainArgName, `other_${mainArgName}`]
    });
    includesFirst.add('#include <stdint.h>');
  }

  const hashOperator = interfaceJson.hashOperator;
  if (hashOperator.defined) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    publicOperators += `size_t PSM_API ${nameWithoutTSuffix}_hash(const ${name}* ${mainArgName});\n`;
    funcAliases.push({
      name: `${nameWithoutTSuffix}_hash`,
      args: [mainArgName]
    });
    includesFirst.add('#include <stddef.h>');
  }

  const toStringOperator = interfaceJson.toStringOperator;
  if (toStringOperator.defined) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    publicOperators += `const char* PSM_API ${nameWithoutTSuffix}_to_string(const ${name}* ${mainArgName});\n`;
    publicOperators += `void PSM_API ${nameWithoutTSuffix}_to_string_free(const char* ${mainArgName}_string);\n`;
    funcAliases.push({
      name: `${nameWithoutTSuffix}_to_string`,
      args: [mainArgName]
    });
    funcAliases.push({
      name: `${nameWithoutTSuffix}_to_string_free`,
      args: [`${mainArgName}_string`]
    });
  }

  for (const propertyJson of util.getProperties(interfaceJson)) {
    let shouldAddIncludes = false;
    const propStatic = util.getPropertyStatic(propertyJson);
    if (propertyJson.hasGetter) {
      const getterName = util.getPropertyGetterName(propertyJson, util.C);
      const getterType = util.getPropertyTypeForGetter(enums, interfaces, propertyJson, util.C);
      const getterConstPfx = propertyJson.getterConst ? 'const ' : '';
      const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
      const setterArgName = util.getPropertySetterArgName(propertyJson, util.C);
      const isArray = util.isArrayOfAnyType(propertyJson.type);
      const isData = propertyJson.type === 'data';
      const getterExt = isArray ? `${propStatic ? '' : ', '}uint64_t* out_length` : (isData ? `${propStatic ? '' : ', '}uint64_t* out_size` : '');
      const getter = `${getterType} PSM_API ${nameWithoutTSuffix}_${getterName}(${propStatic ? `` : `${getterConstPfx}${name}* ${mainArgName}`}${getterExt});\n`;
      const arrayGetterFreeFuncHasOptionToDestroyContainedClasses = doesArrayGetterFreeFuncHaveOptionToDestroyContainedClasses(interfaces, propertyJson.type);
      const isArrayOfAnyPtrType = util.isArrayPtrType(propertyJson.type) || util.isArrayPtrRefType(propertyJson.type) || util.isArrayPtrMixType(propertyJson.type);
      const getterFreeExt = arrayGetterFreeFuncHasOptionToDestroyContainedClasses ? (isArrayOfAnyPtrType ? ', uint8_t destroy_contained_class_refs' : ', uint8_t destroy_contained_classes') : '';
      const getterFree = `void PSM_API ${nameWithoutTSuffix}_${getterName}_free(${getterType} ${setterArgName}${getterFreeExt});\n`;
      const getterVisibility = util.getPropertyGetterVisibility(propertyJson);
      if (getterVisibility === util.Visibility.public) {
        shouldAddIncludes = true;
        if (propStatic) {
          let args = [];
          if (isArray) {
            args.push('out_length');
          } else if (isData) {
            args.push('out_size');
          }
          publicFuncs += getter;
          funcAliases.push({
            name: `${nameWithoutTSuffix}_${getterName}`,
            args: args
          });
          if (propertyJson.type === 'string' || util.isClassType(propertyJson.type) || util.isClassPtrType(propertyJson.type) || doesArrayTypeNeedFree(interfaces, propertyJson.type)) {
            args = [setterArgName];
            if (arrayGetterFreeFuncHasOptionToDestroyContainedClasses) {
              if (isArrayOfAnyPtrType) {
                args.push('destroy_contained_class_refs');
              } else {
                args.push('destroy_contained_classes');
              }
            }
            publicFuncs += getterFree;
            funcAliases.push({
              name: `${nameWithoutTSuffix}_${getterName}_free`,
              args: args
            });
          }
        } else {
          let args = [mainArgName];
          if (isArray) {
            args.push('out_length');
          } else if (isData) {
            args.push('out_size');
          }
          publicMethods += getter;
          funcAliases.push({
            name: `${nameWithoutTSuffix}_${getterName}`,
            args: args
          });
          if (propertyJson.type === 'string' || util.isClassType(propertyJson.type) || util.isClassPtrType(propertyJson.type) || doesArrayTypeNeedFree(interfaces, propertyJson.type)) {
            args = [setterArgName];
            if (arrayGetterFreeFuncHasOptionToDestroyContainedClasses) {
              if (isArrayOfAnyPtrType) {
                args.push('destroy_contained_class_refs');
              } else {
                args.push('destroy_contained_classes');
              }
            }
            publicMethods += getterFree;
            funcAliases.push({
              name: `${nameWithoutTSuffix}_${getterName}_free`,
              args: args
            });
          }
        }
      }
    }
    if (propertyJson.hasSetter) {
      const setterName = util.getPropertySetterName(propertyJson, util.C);
      const setterType = util.getPropertyTypeForSetter(enums, interfaces, propertyJson, util.C);
      const setterConstPfx = propertyJson.setterConst ? 'const ' : '';
      const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
      let setterArgName = util.getPropertySetterArgName(propertyJson, util.C);
      if (util.isClassType(propertyJson.type) || util.isClassMixType(propertyJson.type) || util.isClassPtrType(propertyJson.type) || util.isClassPtrMixType(propertyJson.type) || ((util.isArrayType(propertyJson.type) || util.isArrayMixType(propertyJson.type) || util.isArrayPtrType(propertyJson.type) || util.isArrayPtrMixType(propertyJson.type)) && typeof interfaces[propertyJson.type.split(':').slice(1).join(':')] !== 'undefined')) {
        setterArgName += '_consumed';
      }
      const isArray = util.isArrayOfAnyType(propertyJson.type);
      const isData = propertyJson.type === 'data';
      const setterExt = isArray ? `, uint64_t length` : (isData ? `, uint64_t size` : '');
      const setter = `void PSM_API ${nameWithoutTSuffix}_${setterName}(${propStatic ? `` : `${setterConstPfx}${name}* ${mainArgName}, `}${setterType} ${setterArgName}${setterExt});\n`;
      const setterVisibility = util.getPropertySetterVisibility(propertyJson);
      if (setterVisibility === util.Visibility.public) {
        shouldAddIncludes = true;
        if (propStatic) {
          publicFuncs += setter;
          funcAliases.push({
            name: `${nameWithoutTSuffix}_${setterName}`,
            args: isArray ? [setterArgName, 'length'] : (isData ? [setterArgName, 'size'] : [setterArgName])
          });
        } else {
          publicMethods += setter;
          funcAliases.push({
            name: `${nameWithoutTSuffix}_${setterName}`,
            args: isArray ? [mainArgName, setterArgName, 'length'] : (isData ? [mainArgName, setterArgName, 'size'] : [mainArgName, setterArgName])
          });
        }
      }
    }
    if (shouldAddIncludes) {
      const propTypeRaw = propertyJson.type;
      if (util.isIntType(propTypeRaw) || propTypeRaw === 'boolean') {
        includesFirst.add('#include <stdint.h>');
      } else if (util.isEnumType(propTypeRaw)) {
        includesSecond.add(`#include "${propTypeRaw.split(':').slice(1).join(':')}.h"`);
      } else if (util.isClassOfAnyType(propTypeRaw)) {
        includesSecond.add(`#include "${propTypeRaw.split(':').slice(1).join(':')}.h"`);
      } else if (util.isArrayOfAnyType(propTypeRaw)) {
        includesFirst.add('#include <stdint.h>');
        const underlyingArrayTypeRaw = propTypeRaw.split(':').slice(1).join(':');
        if (typeof enums[underlyingArrayTypeRaw] !== 'undefined' || typeof interfaces[underlyingArrayTypeRaw] !== 'undefined') {
          includesSecond.add(`#include "${underlyingArrayTypeRaw}.h"`);
        }
      }
    }
  }

  for (const methodJson of interfaceJson.methods) {
    
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
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    code += `\n`;
    code += `${nameRef}* PSM_API ${nameRefWithoutTSuffix}_make(${name}* ${mainArgName});\n`;
    funcAliases.push({
      name: `${nameRefWithoutTSuffix}_make`,
      args: [mainArgName]
    });
    code += `${nameRef}* PSM_API ${nameRefWithoutTSuffix}_clone(const ${nameRef}* ${mainArgName}_ref);\n`;
    funcAliases.push({
      name: `${nameRefWithoutTSuffix}_clone`,
      args: [`${mainArgName}_ref`]
    });
    code += `${name}* PSM_API ${nameRefWithoutTSuffix}_get(const ${nameRef}* ${mainArgName}_ref);\n`;
    funcAliases.push({
      name: `${nameRefWithoutTSuffix}_get`,
      args: [`${mainArgName}_ref`]
    });
    code += `void PSM_API ${nameRefWithoutTSuffix}_delete(${nameRef}* ${mainArgName}_ref);\n`;
    funcAliases.push({
      name: `${nameRefWithoutTSuffix}_delete`,
      args: [`${mainArgName}_ref`]
    });
  }

  code += `\n`;
  code += '#if defined(__cplusplus)\n';
  code += '}\n';
  code += '#endif\n';

  if (interfaceJson['c.generateFuncAliasDefines'] && funcAliases.length > 0) {
    for (const alias of util.getLangAliases(interfaceJson, util.C)) {
      const aliasWithoutTSuffix = alias.substring(0, alias.length - 2);
      code += '\n';
      for (const funcAlias of funcAliases) {
        let args = '';
        let argsSent = '';
        for (const arg of funcAlias.args) {
          if (args.length > 0) {
            args += ', ';
            argsSent += ', ';
          }
          args += `_${arg}`;
          argsSent += `(_${arg})`;
        }
        code += `#define ${aliasWithoutTSuffix}${funcAlias.name.substring(nameWithoutTSuffix.length)}(${args}) (${funcAlias.name}(${argsSent}))\n`;
      }
    }
  }

  code += `\n`;
  code += `#endif /* ${headerGuard} */\n`;

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

function generateSource(enums, interfaces, interfaceName, interfaceJson) {
  const name = util.getLangClassName(interfaceJson, util.C);
  const nameWithoutTSuffix = name.substring(0, name.length - 2);
  const nameRefWithoutTSuffix = `${nameWithoutTSuffix}_ref`;
  const nameRef = `${nameRefWithoutTSuffix}_t`;
  const nameCxx = util.getLangClassName(interfaceJson, util.CXX);
  const static = util.getClassStatic(interfaceJson);
  const copyable = util.getClassCopyable(interfaceJson);

  let includesFirst = new Set([`#include <Posemesh/C/${interfaceName}.h>`, `#include <Posemesh/${interfaceName}.hpp>`]), includesSecond = new Set([]);

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += '%INCLUDES%';

  let publicCtors = '', publicOperators = '', publicMethods = '', publicFuncs = '';
  let privateCtors = '', privateOperators = '', privateMethods = '', privateFuncs = '';

  const parameterlessConstructor = util.getClassParameterlessConstructor(interfaceJson);
  const pCtorDefinition = util.getConstructorDefinition(parameterlessConstructor);
  const pCtorVisibility = util.getConstructorVisibility(parameterlessConstructor);
  if (!static && pCtorDefinition !== util.ConstructorDefinition.deleted) {
    let pCtor = `${name}* ${nameWithoutTSuffix}_create()\n`;
    pCtor += `{\n`;
    pCtor += `    return new (std::nothrow) psm::${nameCxx};\n`;
    pCtor += `}\n`;
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

  const copyConstructor = util.getClassCopyConstructor(interfaceJson);
  const cCtorDefinition = util.getConstructorDefinition(copyConstructor);
  const cCtorVisibility = util.getConstructorVisibility(copyConstructor);
  if (!static && copyable && cCtorDefinition !== util.ConstructorDefinition.deleted) {
    const mainArgName = util.getCopyOrMoveConstructorMainArgName(copyConstructor, util.C);
    let cCtor = `${name}* ${nameWithoutTSuffix}_duplicate(const ${name}* ${mainArgName})\n`;
    cCtor += `{\n`;
    cCtor += `    if (!${mainArgName}) {\n`;
    cCtor += `        assert(!"${nameWithoutTSuffix}_duplicate(): ${mainArgName} is null");\n`;
    cCtor += `        return nullptr;\n`;
    cCtor += `    }\n`;
    cCtor += `    return new (std::nothrow) psm::${nameCxx}(*${mainArgName});\n`;
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

    includesFirst.add('#include <cassert>');
    includesFirst.add('#include <new>');
  }

  if (!static) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    let dtor = `void ${nameWithoutTSuffix}_destroy(${name}* ${mainArgName})\n`;
    dtor += `{\n`;
    dtor += `    delete ${mainArgName};\n`;
    dtor += `}\n`;
    if (publicCtors.length > 0) {
      publicCtors += '\n';
    }
    publicCtors += dtor;
  }

  const equalityOperator = interfaceJson.equalityOperator;
  if (equalityOperator.defined) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    let eqOp = `uint8_t ${nameWithoutTSuffix}_equals(const ${name}* ${mainArgName}, const ${name}* other_${mainArgName})\n`;
    eqOp += `{\n`;
    eqOp += `    if (!${mainArgName}) {\n`;
    eqOp += `        assert(!"${nameWithoutTSuffix}_equals(): ${mainArgName} is null");\n`;
    eqOp += `        return 0;\n`;
    eqOp += `    }\n`;
    eqOp += `    if (!other_${mainArgName}) {\n`;
    eqOp += `        assert(!"${nameWithoutTSuffix}_equals(): other_${mainArgName} is null");\n`;
    eqOp += `        return 0;\n`;
    eqOp += `    }\n`;
    eqOp += `    return static_cast<uint8_t>(${mainArgName}->operator==(*other_${mainArgName}));\n`;
    eqOp += `}\n`;

    if (publicOperators.length > 0) {
      publicOperators += '\n';
    }
    publicOperators += eqOp;

    includesFirst.add('#include <cassert>');
  }

  const hashOperator = interfaceJson.hashOperator;
  if (hashOperator.defined) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    let hashOp = `size_t ${nameWithoutTSuffix}_hash(const ${name}* ${mainArgName})\n`;
    hashOp += `{\n`;
    hashOp += `    if (!${mainArgName}) {\n`;
    hashOp += `        assert(!"${nameWithoutTSuffix}_hash(): ${mainArgName} is null");\n`;
    hashOp += `        return 0;\n`;
    hashOp += `    }\n`;
    hashOp += `    return std::hash<psm::${nameCxx}> {}(*${mainArgName});\n`;
    hashOp += `}\n`;

    if (publicOperators.length > 0) {
      publicOperators += '\n';
    }
    publicOperators += hashOp;

    includesFirst.add('#include <cassert>');
  }

  const toStringOperator = interfaceJson.toStringOperator;
  if (toStringOperator.defined) {
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    let toStrOp = `const char* ${nameWithoutTSuffix}_to_string(const ${name}* ${mainArgName})\n`;
    toStrOp += `{\n`;
    toStrOp += `    if (!${mainArgName}) {\n`;
    toStrOp += `        assert(!"${nameWithoutTSuffix}_to_string(): ${mainArgName} is null");\n`;
    toStrOp += `        return nullptr;\n`;
    toStrOp += `    }\n`;
    toStrOp += `    const auto ${mainArgName}_string = static_cast<std::string>(*${mainArgName});\n`;
    toStrOp += `    auto* getter_result = new (std::nothrow) char[${mainArgName}_string.size() + 1];\n`;
    toStrOp += `    if (!getter_result) {\n`;
    toStrOp += `        return nullptr;\n`;
    toStrOp += `    }\n`;
    toStrOp += `    std::memcpy(getter_result, ${mainArgName}_string.c_str(), ${mainArgName}_string.size() + 1);\n`;
    toStrOp += `    return getter_result;\n`;
    toStrOp += `}\n`;
    toStrOp += `\n`;
    toStrOp += `void ${nameWithoutTSuffix}_to_string_free(const char* ${mainArgName}_string)\n`;
    toStrOp += `{\n`;
    toStrOp += `    delete[] const_cast<char*>(${mainArgName}_string);\n`;
    toStrOp += `}\n`;

    if (publicOperators.length > 0) {
      publicOperators += '\n';
    }
    publicOperators += toStrOp;

    includesFirst.add('#include <cassert>');
    includesFirst.add('#include <cstring>');
    includesFirst.add('#include <new>');
  }

  for (const propertyJson of util.getProperties(interfaceJson)) {
    const propTypeRaw = propertyJson.type;
    const propStatic = util.getPropertyStatic(propertyJson);
    if (propertyJson.hasGetter) {
      const getterName = util.getPropertyGetterName(propertyJson, util.C);
      const getterType = util.getPropertyTypeForGetter(enums, interfaces, propertyJson, util.C);
      const getterConstPfx = propertyJson.getterConst ? 'const ' : '';
      const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
      const setterArgName = util.getPropertySetterArgName(propertyJson, util.C);
      const isArray = util.isArrayOfAnyType(propertyJson.type);
      const isData = propertyJson.type === 'data';
      const getterExt = isArray ? `${propStatic ? '' : ', '}uint64_t* out_length` : (isData ? `${propStatic ? '' : ', '}uint64_t* out_size` : '');
      let getter = `${getterType} ${nameWithoutTSuffix}_${getterName}(${propStatic ? `` : `${getterConstPfx}${name}* ${mainArgName}`}${getterExt})\n`;
      const arrayGetterFreeFuncHasOptionToDestroyContainedClasses = doesArrayGetterFreeFuncHaveOptionToDestroyContainedClasses(interfaces, propertyJson.type);
      const isArrayOfAnyPtrType = util.isArrayPtrType(propertyJson.type) || util.isArrayPtrRefType(propertyJson.type) || util.isArrayPtrMixType(propertyJson.type);
      const getterFreeExt = arrayGetterFreeFuncHasOptionToDestroyContainedClasses ? (isArrayOfAnyPtrType ? ', uint8_t destroy_contained_class_refs' : ', uint8_t destroy_contained_classes') : '';
      getter += `{\n`;
      if (propStatic) {
        if (propTypeRaw === 'boolean') {
          getter += `    return static_cast<uint8_t>(psm::${nameCxx}::${util.getPropertyGetterName(propertyJson, util.CXX)}());\n`;
        } else if (propTypeRaw === 'string') {
          getter += `    const auto ${setterArgName} = psm::${nameCxx}::${util.getPropertyGetterName(propertyJson, util.CXX)}();\n`;
          getter += `    auto* getter_result = new (std::nothrow) char[${setterArgName}.size() + 1];\n`;
          getter += `    if (!getter_result) {\n`;
          getter += `        return nullptr;\n`;
          getter += `    }\n`;
          getter += `    std::memcpy(getter_result, ${setterArgName}.c_str(), ${setterArgName}.size() + 1);\n`;
          getter += `    return getter_result;\n`;
        } else if (propTypeRaw === 'string_ref' || propTypeRaw === 'string_mix') {
          getter += `    return psm::${nameCxx}::${util.getPropertyGetterName(propertyJson, util.CXX)}().c_str();\n`;
        } else if (util.isEnumType(propTypeRaw)) {
          getter += `    return static_cast<${getterType}>(psm::${nameCxx}::${util.getPropertyGetterName(propertyJson, util.CXX)}());\n`;
        } else if (util.isClassType(propTypeRaw)) {
          getter += `    const auto getter_result = psm::${nameCxx}::${util.getPropertyGetterName(propertyJson, util.CXX)}();\n`;
          getter += `    return ${getterType.substring(0, getterType.length - 3)}_duplicate(&getter_result);\n`;
        } else if (util.isClassRefType(propTypeRaw) || util.isClassMixType(propTypeRaw)) {
          getter += `    const auto& getter_result = psm::${nameCxx}::${util.getPropertyGetterName(propertyJson, util.CXX)}();\n`;
          getter += `    return &getter_result;\n`;
        } else if (util.isClassPtrType(propTypeRaw)) {
          getter += `    const auto getter_result = psm::${nameCxx}::${util.getPropertyGetterName(propertyJson, util.CXX)}();\n`;
          getter += `    return getter_result ? ${getterType.substring(0, getterType.length - 3)}_clone(&getter_result) : nullptr;\n`;
        } else if (util.isClassPtrRefType(propTypeRaw) || util.isClassPtrMixType(propTypeRaw)) {
          getter += `    const auto& getter_result = psm::${nameCxx}::${util.getPropertyGetterName(propertyJson, util.CXX)}();\n`;
          getter += `    return getter_result ? &getter_result : nullptr;\n`;
        } else if (isArray) {
          getter += arrayGetterCode(enums, interfaces, propertyJson, propTypeRaw, mainArgName, setterArgName, nameCxx, propStatic);
        } else if (isData) {
          getter += `    std::size_t size;\n`;
          getter += `    const auto* result = psm::${nameCxx}::${util.getPropertyGetterName(propertyJson, util.CXX)}(size);\n`;
          getter += `    if (out_size) {\n`;
          getter += `        *out_size = static_cast<uint64_t>(size);\n`;
          getter += `    }\n`;
          getter += `    return result;\n`;
        } else {
          getter += `    return psm::${nameCxx}::${util.getPropertyGetterName(propertyJson, util.CXX)}();\n`;
        }
      } else {
        getter += `    if (!${mainArgName}) {\n`;
        getter += `        assert(!"${nameWithoutTSuffix}_${getterName}(): ${mainArgName} is null");\n`;
        if (propTypeRaw === 'boolean') {
          getter += `        return static_cast<uint8_t>(${util.getTypeImplicitDefaultValue(propTypeRaw)});\n`;
        } else if (propTypeRaw === 'string' || propTypeRaw === 'string_ref' || propTypeRaw === 'string_mix') {
          getter += `        return nullptr;\n`;
        } else if (util.isEnumType(propTypeRaw)) {
          getter += `        return static_cast<${getterType}>(0);\n`;
        } else if (util.isClassOfAnyType(propTypeRaw)) {
          getter += `        return nullptr;\n`;
        } else if (isArray) {
          getter += `        if (out_length) {\n`;
          getter += `            *out_length = 0;\n`;
          getter += `        }\n`;
          getter += `        return nullptr;\n`;
        } else if (isData) {
          getter += `        return nullptr;\n`;
        } else {
          getter += `        return ${util.getTypeImplicitDefaultValue(propTypeRaw)};\n`;
        }
        getter += `    }\n`;
        if (propTypeRaw === 'boolean') {
          getter += `    return static_cast<uint8_t>(${mainArgName}->${util.getPropertyGetterName(propertyJson, util.CXX)}());\n`;
        } else if (propTypeRaw === 'string') {
          getter += `    const auto ${setterArgName} = ${mainArgName}->${util.getPropertyGetterName(propertyJson, util.CXX)}();\n`;
          getter += `    auto* getter_result = new (std::nothrow) char[${setterArgName}.size() + 1];\n`;
          getter += `    if (!getter_result) {\n`;
          getter += `        return nullptr;\n`;
          getter += `    }\n`;
          getter += `    std::memcpy(getter_result, ${setterArgName}.c_str(), ${setterArgName}.size() + 1);\n`;
          getter += `    return getter_result;\n`;
        } else if (propTypeRaw === 'string_ref' || propTypeRaw === 'string_mix') {
          getter += `    return ${mainArgName}->${util.getPropertyGetterName(propertyJson, util.CXX)}().c_str();\n`;
        } else if (util.isEnumType(propTypeRaw)) {
          getter += `    return static_cast<${getterType}>(${mainArgName}->${util.getPropertyGetterName(propertyJson, util.CXX)}());\n`;
        } else if (util.isClassType(propTypeRaw)) {
          getter += `    const auto getter_result = ${mainArgName}->${util.getPropertyGetterName(propertyJson, util.CXX)}();\n`;
          getter += `    return ${getterType.substring(0, getterType.length - 3)}_duplicate(&getter_result);\n`;
        } else if (util.isClassRefType(propTypeRaw) || util.isClassMixType(propTypeRaw)) {
          getter += `    const auto& getter_result = ${mainArgName}->${util.getPropertyGetterName(propertyJson, util.CXX)}();\n`;
          getter += `    return &getter_result;\n`;
        } else if (util.isClassPtrType(propTypeRaw)) {
          getter += `    const auto getter_result = ${mainArgName}->${util.getPropertyGetterName(propertyJson, util.CXX)}();\n`;
          getter += `    return getter_result ? ${getterType.substring(0, getterType.length - 3)}_clone(&getter_result) : nullptr;\n`;
        } else if (util.isClassPtrRefType(propTypeRaw) || util.isClassPtrMixType(propTypeRaw)) {
          getter += `    const auto& getter_result = ${mainArgName}->${util.getPropertyGetterName(propertyJson, util.CXX)}();\n`;
          getter += `    return getter_result ? &getter_result : nullptr;\n`;
        } else if (isArray) {
          getter += arrayGetterCode(enums, interfaces, propertyJson, propTypeRaw, mainArgName, setterArgName, nameCxx, propStatic);
        } else if (isData) {
          getter += `    std::size_t size;\n`;
          getter += `    const auto* result = ${mainArgName}->${util.getPropertyGetterName(propertyJson, util.CXX)}(size);\n`;
          getter += `    if (out_size) {\n`;
          getter += `        *out_size = static_cast<uint64_t>(size);\n`;
          getter += `    }\n`;
          getter += `    return result;\n`;
        } else {
          getter += `    return ${mainArgName}->${util.getPropertyGetterName(propertyJson, util.CXX)}();\n`;
        }
      }
      getter += `}\n`;
      if (propTypeRaw === 'string' || util.isClassType(propTypeRaw) || util.isClassPtrType(propTypeRaw) || doesArrayTypeNeedFree(interfaces, propTypeRaw)) {
        getter += `\n`;
        getter += `void ${nameWithoutTSuffix}_${getterName}_free(${getterType} ${setterArgName}${getterFreeExt})\n`;
        getter += `{\n`;
        if (propTypeRaw === 'string') {
          getter += `    delete[] const_cast<char*>(${setterArgName});\n`;
        } else if (util.isClassType(propTypeRaw)) {
          getter += `    ${getterType.substring(0, getterType.length - 3)}_destroy(${setterArgName});\n`;
        } else if (util.isClassPtrType(propTypeRaw)) {
          getter += `    ${getterType.substring(0, getterType.length - 3)}_delete(${setterArgName});\n`;
        } else if (isArray) {
          if (arrayGetterFreeFuncHasOptionToDestroyContainedClasses) {
            getter += `    if (!${setterArgName}) {\n`;
            getter += `        return;\n`;
            getter += `    }\n`;
            getter += `    if (${isArrayOfAnyPtrType ? 'destroy_contained_class_refs' : 'destroy_contained_classes'}) {\n`;
            getter += `        const std::size_t length = reinterpret_cast<std::size_t>(*(${setterArgName} - 1));\n`;
            getter += `        for (std::size_t i = 0; i < length; ++i) {\n`;
            getter += `            delete ${setterArgName}[i];\n`;
            getter += `        }\n`;
            getter += `    }\n`;
            getter += `    delete[] (${setterArgName} - 1);\n`;
          } else {
            getter += `    delete[] ${setterArgName};\n`;
          }
        } else {
          getter += '    #error "Not implemented."\n';
        }
        getter += `}\n`;
      }
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
          includesFirst.add('#include <cassert>');
        }
        if (propTypeRaw === 'string') {
          includesFirst.add('#include <cstring>');
          includesFirst.add('#include <new>');
        } else if (isArray) {
          arrayGetterIncludes(enums, interfaces, propTypeRaw, includesFirst, includesSecond);
        }
      }
    }
    if (propertyJson.hasSetter) {
      const setterName = util.getPropertySetterName(propertyJson, util.C);
      const setterType = util.getPropertyTypeForSetter(enums, interfaces, propertyJson, util.C);
      const setterTypeCxx = util.getPropertyTypeForSetter(enums, interfaces, propertyJson, util.CXX);
      const setterConstPfx = propertyJson.setterConst ? 'const ' : '';
      const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
      let setterArgName = util.getPropertySetterArgName(propertyJson, util.C);
      if (util.isClassType(propertyJson.type) || util.isClassMixType(propertyJson.type) || util.isClassPtrType(propertyJson.type) || util.isClassPtrMixType(propertyJson.type) || ((util.isArrayType(propertyJson.type) || util.isArrayMixType(propertyJson.type) || util.isArrayPtrType(propertyJson.type) || util.isArrayPtrMixType(propertyJson.type)) && typeof interfaces[propertyJson.type.split(':').slice(1).join(':')] !== 'undefined')) {
        setterArgName += '_consumed';
      }
      const isArray = util.isArrayOfAnyType(propertyJson.type);
      const isData = propertyJson.type === 'data';
      const setterExt = isArray ? `, uint64_t length` : (isData ? `, uint64_t size` : '');
      let setter = `void ${nameWithoutTSuffix}_${setterName}(${propStatic ? `` : `${setterConstPfx}${name}* ${mainArgName}, `}${setterType} ${setterArgName}${setterExt})\n`;
      setter += `{\n`;
      if (propStatic) {
        if (propTypeRaw === 'boolean') {
          setter += `    psm::${nameCxx}::${util.getPropertySetterName(propertyJson, util.CXX)}(static_cast<bool>(${setterArgName}));\n`;
        } else if (propTypeRaw === 'string' || propTypeRaw === 'string_ref' || propTypeRaw === 'string_mix') {
          setter += `    psm::${nameCxx}::${util.getPropertySetterName(propertyJson, util.CXX)}(${setterArgName} ? std::string { ${setterArgName} } : std::string {});\n`;
        } else if (util.isEnumType(propTypeRaw)) {
          setter += `    psm::${nameCxx}::${util.getPropertySetterName(propertyJson, util.CXX)}(static_cast<psm::${setterTypeCxx}>(${setterArgName}));\n`;
        } else if (util.isClassType(propTypeRaw)) {
          setter += `    if (!${setterArgName}) {\n`;
          setter += `        assert(!"${nameWithoutTSuffix}_${setterName}(): ${setterArgName} is null");\n`;
          setter += `        return;\n`;
          setter += `    }\n`;
          setter += `    const std::unique_ptr<${setterType.substring(0, setterType.length - 1)}, decltype(&${setterType.substring(0, setterType.length - 3)}_destroy)> raii_wrapper(${setterArgName}, &${setterType.substring(0, setterType.length - 3)}_destroy);\n`;
          setter += `    psm::${nameCxx}::${util.getPropertySetterName(propertyJson, util.CXX)}(std::move(*raii_wrapper));\n`;
        } else if (util.isClassRefType(propTypeRaw)) {
          setter += `    if (!${setterArgName}) {\n`;
          setter += `        assert(!"${nameWithoutTSuffix}_${setterName}(): ${setterArgName} is null");\n`;
          setter += `        return;\n`;
          setter += `    }\n`;
          setter += `    psm::${nameCxx}::${util.getPropertySetterName(propertyJson, util.CXX)}(*${setterArgName});\n`;
        } else if (util.isClassMixType(propTypeRaw)) {
          setter += `    if (!${setterArgName}) {\n`;
          setter += `        assert(!"${nameWithoutTSuffix}_${setterName}(): ${setterArgName} is null");\n`;
          setter += `        return;\n`;
          setter += `    }\n`;
          setter += `    const std::unique_ptr<${setterType.substring(0, setterType.length - 1)}, decltype(&${setterType.substring(0, setterType.length - 3)}_destroy)> raii_wrapper(${setterArgName}, &${setterType.substring(0, setterType.length - 3)}_destroy);\n`;
          setter += `    psm::${nameCxx}::${util.getPropertySetterName(propertyJson, util.CXX)}(std::move(*raii_wrapper));\n`;
        } else if (util.isClassPtrType(propTypeRaw)) {
          setter += `    const std::unique_ptr<${setterType.substring(0, setterType.length - 1)}, decltype(&${setterType.substring(0, setterType.length - 3)}_delete)> raii_wrapper(${setterArgName}, &${setterType.substring(0, setterType.length - 3)}_delete);\n`;
          setter += `    psm::${nameCxx}::${util.getPropertySetterName(propertyJson, util.CXX)}(raii_wrapper ? ${setterType.substring(0, setterType.length - 1)} { std::move(*raii_wrapper) } : ${setterType.substring(0, setterType.length - 1)} {});\n`;
        } else if (util.isClassPtrRefType(propTypeRaw)) {
          setter += `    psm::${nameCxx}::${util.getPropertySetterName(propertyJson, util.CXX)}(${setterArgName} ? ${setterType.substring(6/*!!!*/, setterType.length - 1)} { *${setterArgName} } : ${setterType.substring(6/*!!!*/, setterType.length - 1)} {});\n`;
        } else if (util.isClassPtrMixType(propTypeRaw)) {
          setter += `    const std::unique_ptr<${setterType.substring(0, setterType.length - 1)}, decltype(&${setterType.substring(0, setterType.length - 3)}_delete)> raii_wrapper(${setterArgName}, &${setterType.substring(0, setterType.length - 3)}_delete);\n`;
          setter += `    psm::${nameCxx}::${util.getPropertySetterName(propertyJson, util.CXX)}(raii_wrapper ? ${setterType.substring(0, setterType.length - 1)} { std::move(*raii_wrapper) } : ${setterType.substring(0, setterType.length - 1)} {});\n`;
        } else if (isArray) {
          setter += arraySetterCode(enums, interfaces, propertyJson, propTypeRaw, mainArgName, setterName, setterArgName, nameCxx, nameWithoutTSuffix, propStatic);
        } else if (isData) {
          setter += `    psm::${nameCxx}::${util.getPropertySetterName(propertyJson, util.CXX)}(${setterArgName}, static_cast<std::size_t>(size));\n`;
        } else {
          setter += `    psm::${nameCxx}::${util.getPropertySetterName(propertyJson, util.CXX)}(${setterArgName});\n`;
        }
      } else {
        setter += `    if (!${mainArgName}) {\n`;
        setter += `        assert(!"${nameWithoutTSuffix}_${setterName}(): ${mainArgName} is null");\n`;
        setter += `        return;\n`;
        setter += `    }\n`;
        if (propTypeRaw === 'boolean') {
          setter += `    ${mainArgName}->${util.getPropertySetterName(propertyJson, util.CXX)}(static_cast<bool>(${setterArgName}));\n`;
        } else if (propTypeRaw === 'string' || propTypeRaw === 'string_ref' || propTypeRaw === 'string_mix') {
          setter += `    ${mainArgName}->${util.getPropertySetterName(propertyJson, util.CXX)}(${setterArgName} ? std::string { ${setterArgName} } : std::string {});\n`;
        } else if (util.isEnumType(propTypeRaw)) {
          setter += `    ${mainArgName}->${util.getPropertySetterName(propertyJson, util.CXX)}(static_cast<psm::${setterTypeCxx}>(${setterArgName}));\n`;
        } else if (util.isClassType(propTypeRaw)) {
          setter += `    if (!${setterArgName}) {\n`;
          setter += `        assert(!"${nameWithoutTSuffix}_${setterName}(): ${setterArgName} is null");\n`;
          setter += `        return;\n`;
          setter += `    }\n`;
          setter += `    const std::unique_ptr<${setterType.substring(0, setterType.length - 1)}, decltype(&${setterType.substring(0, setterType.length - 3)}_destroy)> raii_wrapper(${setterArgName}, &${setterType.substring(0, setterType.length - 3)}_destroy);\n`;
          setter += `    ${mainArgName}->${util.getPropertySetterName(propertyJson, util.CXX)}(std::move(*raii_wrapper));\n`;
        } else if (util.isClassRefType(propTypeRaw)) {
          setter += `    if (!${setterArgName}) {\n`;
          setter += `        assert(!"${nameWithoutTSuffix}_${setterName}(): ${setterArgName} is null");\n`;
          setter += `        return;\n`;
          setter += `    }\n`;
          setter += `    ${mainArgName}->${util.getPropertySetterName(propertyJson, util.CXX)}(*${setterArgName});\n`;
        } else if (util.isClassMixType(propTypeRaw)) {
          setter += `    if (!${setterArgName}) {\n`;
          setter += `        assert(!"${nameWithoutTSuffix}_${setterName}(): ${setterArgName} is null");\n`;
          setter += `        return;\n`;
          setter += `    }\n`;
          setter += `    const std::unique_ptr<${setterType.substring(0, setterType.length - 1)}, decltype(&${setterType.substring(0, setterType.length - 3)}_destroy)> raii_wrapper(${setterArgName}, &${setterType.substring(0, setterType.length - 3)}_destroy);\n`;
          setter += `    ${mainArgName}->${util.getPropertySetterName(propertyJson, util.CXX)}(std::move(*raii_wrapper));\n`;
        } else if (util.isClassPtrType(propTypeRaw)) {
          setter += `    const std::unique_ptr<${setterType.substring(0, setterType.length - 1)}, decltype(&${setterType.substring(0, setterType.length - 3)}_delete)> raii_wrapper(${setterArgName}, &${setterType.substring(0, setterType.length - 3)}_delete);\n`;
          setter += `    ${mainArgName}->${util.getPropertySetterName(propertyJson, util.CXX)}(raii_wrapper ? ${setterType.substring(0, setterType.length - 1)} { std::move(*raii_wrapper) } : ${setterType.substring(0, setterType.length - 1)} {});\n`;
        } else if (util.isClassPtrRefType(propTypeRaw)) {
          setter += `    ${mainArgName}->${util.getPropertySetterName(propertyJson, util.CXX)}(${setterArgName} ? ${setterType.substring(6/*!!!*/, setterType.length - 1)} { *${setterArgName} } : ${setterType.substring(6/*!!!*/, setterType.length - 1)} {});\n`;
        } else if (util.isClassPtrMixType(propTypeRaw)) {
          setter += `    const std::unique_ptr<${setterType.substring(0, setterType.length - 1)}, decltype(&${setterType.substring(0, setterType.length - 3)}_delete)> raii_wrapper(${setterArgName}, &${setterType.substring(0, setterType.length - 3)}_delete);\n`;
          setter += `    ${mainArgName}->${util.getPropertySetterName(propertyJson, util.CXX)}(raii_wrapper ? ${setterType.substring(0, setterType.length - 1)} { std::move(*raii_wrapper) } : ${setterType.substring(0, setterType.length - 1)} {});\n`;
        } else if (isArray) {
          setter += arraySetterCode(enums, interfaces, propertyJson, propTypeRaw, mainArgName, setterName, setterArgName, nameCxx, nameWithoutTSuffix, propStatic);
        } else if (isData) {
          setter += `    ${mainArgName}->${util.getPropertySetterName(propertyJson, util.CXX)}(${setterArgName}, static_cast<std::size_t>(size));\n`;
        } else {
          setter += `    ${mainArgName}->${util.getPropertySetterName(propertyJson, util.CXX)}(${setterArgName});\n`;
        }
      }
      setter += `}\n`;
      const setterVisibility = util.getPropertySetterVisibility(propertyJson);
      if (setterVisibility === util.Visibility.public) {
        if (propStatic) {
          if (publicFuncs.length > 0) {
            publicFuncs += '\n';
          }
          publicFuncs += setter;
          if (util.isClassOfAnyType(propTypeRaw)) {
            includesFirst.add('#include <cassert>');
          }
        } else {
          if (publicMethods.length > 0) {
            publicMethods += '\n';
          }
          publicMethods += setter;
          includesFirst.add('#include <cassert>');
        }
        if (util.isClassType(propTypeRaw) || util.isClassMixType(propTypeRaw) || util.isClassPtrType(propTypeRaw) || util.isClassPtrMixType(propTypeRaw)) {
          includesFirst.add('#include <memory>');
          includesFirst.add('#include <utility>');
        } else if (isArray) {
          arraySetterIncludes(enums, interfaces, propTypeRaw, includesFirst, includesSecond);
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
    const mainArgName = util.getStyleName('name', interfaceJson, util.lower_case);
    code += `\n`;
    code += `${nameRef}* ${nameRefWithoutTSuffix}_make(${name}* ${mainArgName})\n`;
    code += `{\n`;
    code += `    return new (std::nothrow) ${nameRef}(${mainArgName}, &${nameWithoutTSuffix}_destroy);\n`;
    code += `}\n`;

    includesFirst.add('#include <new>');

    code += `\n`;
    code += `${nameRef}* ${nameRefWithoutTSuffix}_clone(const ${nameRef}* ${mainArgName}_ref)\n`;
    code += `{\n`;
    code += `    if (!${mainArgName}_ref) {\n`;
    code += `        return nullptr;\n`;
    code += `    }\n`;
    code += `    return new (std::nothrow) ${nameRef}(*${mainArgName}_ref);\n`;
    code += `}\n`;

    includesFirst.add('#include <new>');

    code += `\n`;
    code += `${name}* ${nameRefWithoutTSuffix}_get(const ${nameRef}* ${mainArgName}_ref)\n`;
    code += `{\n`;
    code += `    if (!${mainArgName}_ref) {\n`;
    code += `        return nullptr;\n`;
    code += `    }\n`;
    code += `    return ${mainArgName}_ref->get();\n`;
    code += `}\n`;

    code += `\n`;
    code += `void ${nameRefWithoutTSuffix}_delete(${nameRef}* ${mainArgName}_ref)\n`;
    code += `{\n`;
    code += `    delete ${mainArgName}_ref;\n`;
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

function generateInterfaceC(enums, interfaces, interfaceName, interfaceJson) {
  const headerFilePath = path.resolve(__dirname, '..', 'include', 'Posemesh', 'C', `${interfaceName}.h`);
  const sourceFilePath = path.resolve(__dirname, '..', 'src', 'C', `${interfaceName}.cpp`);

  let headerCode = generateHeader(enums, interfaces, interfaceName, interfaceJson);
  let sourceCode = generateSource(enums, interfaces, interfaceName, interfaceJson);

  util.writeFileContentIfDifferent(headerFilePath, headerCode);
  util.writeFileContentIfDifferent(sourceFilePath, sourceCode);
}

module.exports = generateInterfaceC;
