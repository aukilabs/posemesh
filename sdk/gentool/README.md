# GenTool

## JSON documentation

### Root JSON options

| Name                       | Required | Type                       | Description |
|----------------------------|----------|----------------------------|-------------|
| `name`                     | &#x2705; | *string*                   | Name of the class in `Camel_Snake_Case` naming convention. The different [naming conventions](#naming-conventions) are automatically derived from this one, however they can be manually specified with options such as `name.style.lowerCase`, `name.style.upperCase` and such. Furthermore, different [language](#languages) binding names are taken from the generated naming conventions, but they can also be manually specified with options such as `name.lang.cPlusPlus`, `name.lang.c` and such. |
| `static`                   |          | *boolean*                  | Determine whether the class will be static or not. Default is `false`. |
| `final`                    |          | *boolean*                  | Determine whether the class will be final or not. Inferred from `static` option. |
| `aliases`                  |          | *Alias[]*                  | Alternative names of the class. No aliases by default. See [definition](#alias-json-options). |
| `headerGuardName`          |          | *string*                   | Name of the class in `UPPER_CASE` naming convention used for C++ and C header guards. Derived from `name` option. |
| `properties`               |          | *Property[]*               | Properties of the class. No properties by default. See [definition](#property-json-options). |
| `copyable`                 |          | *boolean*                  | Determine whether the class is copyable or not. Inferred from `static` option. |
| `movable`                  |          | *boolean*                  | Determine whether the class is movable or not. Inferred from `static` option. |
| `parameterlessConstructor` |          | *ParameterlessConstructor* | Options for the parameterless constructor. See [definition](#parameterlessconstructor-json-options). |
| `copyConstructor`          |          | *CopyConstructor*          | Options for the copy constructor and operator. See [definition](#copyconstructor-json-options). |
| `moveConstructor`          |          | *MoveConstructor*          | Options for the move constructor and operator. See [definition](#moveconstructor-json-options). |
| `destructor`               |          | *Destructor*               | Options for the destructor. See [definition](#destructor-json-options). |

### Alias JSON options

| Name   | Required | Type      | Description |
|--------|----------|-----------|-------------|
| `name` | &#x2705; | *string*  | Alternative name of the class in `Camel_Snake_Case` naming convention. Same naming convention and language specializations possible as in class `name` option. |

### Property JSON options

| Name               | Required | Type         | Description |
|--------------------|----------|--------------|-------------|
| `name`             | &#x2705; | *string*     | Name of the property in `Camel_Snake_Case` naming convention. Same naming convention and language specializations possible as in class `name` option. |
| `type`             | &#x2705; | *string*     | Type of the property. See [list](#valid-types) of valid types. |
| `static`           |          | *boolean*    | Determine whether the property will be static or not. Inferred from class `static` option. |
| `hasGetter`        |          | *boolean*    | Determine whether the property will have a getter or not. Default is `true`. |
| `getterConst`      |          | *boolean*    | Determine whether the property getter will have a `const` modifier or not. Inferred from class `static` option. |
| `getterNoexcept`   |          | *boolean*    | Determine whether the property getter will have a `noexcept` modifier or not. Inferred from `type` option. |
| `getterName`       |          | *string*     | Name of the property getter in `Camel_Snake_Case` naming convention. Derived from `name` option. Same naming convention and language specializations possible as in class `name` option. |
| `getterMode`       |          | *MethodMode* | Method mode of the property getter. See possible [method modes](#method-modes). Default is `regular`. |
| `getterCustom`     |          | *boolean*    | Determine whether the property getter will be have a custom implementation or not. Inferred from `getterMode` option. |
| `getterVisibility` |          | *Visibility* | Visibility of the property getter. See possible [visibilities](#visibilities). Default is `public`. |
| `hasSetter`        |          | *boolean*    | Determine whether the property will have a setter or not. Default is `true`. |
| `setterConst`      |          | *boolean*    | Determine whether the property setter will have a `const` modifier or not. Inferred from class `static` option. |
| `setterNoexcept`   |          | *boolean*    | Determine whether the property setter will have a `noexcept` modifier or not. Inferred from `type` option. |
| `setterName`       |          | *string*     | Name of the property setter in `Camel_Snake_Case` naming convention. Derived from `name` option. Same naming convention and language specializations possible as in class `name` option. |
| `setterArgName`    |          | *string*     | Name of the property setter argument in `Camel_Snake_Case` naming convention. Derived from `name` option. Same naming convention and language specializations possible as in class `name` option. |
| `setterMode`       |          | *MethodMode* | Method mode of the property setter. See possible [method modes](#method-modes). Default is `regular`. |
| `setterCustom`     |          | *boolean*    | Determine whether the property setter will be have a custom implementation or not. Inferred from `setterMode` option. |
| `setterVisibility` |          | *Visibility* | Visibility of the property setter. See possible [visibilities](#visibilities). Default is `public`. |
| `hasMemberVar`     |          | *boolean*    | Determine whether the property will have a member variable or not. Inferred from `getterCustom` and `setterCustom` options. |
| `defaultValue`     |          | *string*     | Default member variable initialized value either set via a constructor or via a static member initialization if the class is static. Default is empty string. |

### ParameterlessConstructor JSON options

| Name                    | Required | Type                    | Description |
|-------------------------|----------|-------------------------|-------------|
| `initializedProperties` |          | *InitializedProperty[]* | Initialized properties in the constructor. Inferred from class `properties` option. See [definition](#initializedproperty-json-options). |
| `codeFront`             |          | *string[]*              | Constructor code lines placed before the autogenerated constructor body lines. Default is an empty array. |
| `codeBack`              |          | *string[]*              | Constructor code lines placed after the autogenerated constructor body lines. Default is an empty array. |
| `definition`            |          | *ConstructorDefinition* | Determine which constructor definition mode to use. See [possible options](#constructor-definitions). Inferred from class `static` option and constructor context. |
| `visibility`            |          | *Visibility*            | Visibility of the constructor. See possible [visibilities](#visibilities). Inferred from class `static` option. |
| `noexcept`              |          | *boolean*               | Determine whether the constructor will have the `noexcept` modifier or not. Inferred from constructor context and `definition` option. |
| `custom`                |          | *boolean*               | Determine whether the constructor will have a custom implementation or not. Default is `false`. |

### CopyConstructor JSON options

| Name                    | Required | Type                    | Description |
|-------------------------|----------|-------------------------|-------------|
| `mainArgName`           |          | *string*                | Name of the constructor main argument in `Camel_Snake_Case` naming convention. Derived from class `name` option. Same naming convention and language specializations possible as in class `name` option. |
| `initializedProperties` |          | *InitializedProperty[]* | Initialized properties in the constructor. Inferred from class `properties` option. See [definition](#initializedproperty-json-options). |
| `codeFront`             |          | *string[]*              | Constructor code lines placed before the autogenerated constructor body lines. Default is an empty array. |
| `codeBack`              |          | *string[]*              | Constructor code lines placed after the autogenerated constructor body lines. Default is an empty array. |
| `operatorCodeFront`     |          | *string[]*              | Copy assignment operator code lines placed before the autogenerated copy assignment operator body lines. Inferred from `codeFront` option. |
| `operatorCodeBack`      |          | *string[]*              | Copy assignment operator code lines placed after the autogenerated copy assignment operator body lines. Inferred from `codeBack` option. |
| `definition`            |          | *ConstructorDefinition* | Determine which constructor and copy assignment operator definition mode to use. See [possible options](#constructor-definitions). Inferred from class `static` and `copyable` options as well as constructor context. |
| `visibility`            |          | *Visibility*            | Visibility of the constructor and copy assignment operator. See possible [visibilities](#visibilities). Default is `public`. |
| `noexcept`              |          | *boolean*               | Determine whether the constructor and copy assignment operator will have the `noexcept` modifier or not. Inferred from constructor context and `definition` option. |
| `custom`                |          | *boolean*               | Determine whether the constructor will have a custom implementation or not. Default is `false`. |
| `customOperator`        |          | *boolean*               | Determine whether the copy assignment operator will have a custom implementation or not. Inferred from `custom` option. |

### MoveConstructor JSON options

| Name                    | Required | Type                    | Description |
|-------------------------|----------|-------------------------|-------------|
| `mainArgName`           |          | *string*                | Name of the constructor main argument in `Camel_Snake_Case` naming convention. Derived from class `name` option. Same naming convention and language specializations possible as in class `name` option. |
| `initializedProperties` |          | *InitializedProperty[]* | Initialized properties in the constructor. Inferred from class `properties` option. See [definition](#initializedproperty-json-options). |
| `codeFront`             |          | *string[]*              | Constructor code lines placed before the autogenerated constructor body lines. Default is an empty array. |
| `codeBack`              |          | *string[]*              | Constructor code lines placed after the autogenerated constructor body lines. Default is an empty array. |
| `operatorCodeFront`     |          | *string[]*              | Move assignment operator code lines placed before the autogenerated move assignment operator body lines. Inferred from `codeFront` option. |
| `operatorCodeBack`      |          | *string[]*              | Move assignment operator code lines placed after the autogenerated move assignment operator body lines. Inferred from `codeBack` option. |
| `definition`            |          | *ConstructorDefinition* | Determine which constructor and move assignment operator definition mode to use. See [possible options](#constructor-definitions). Inferred from class `static` and `movable` options as well as constructor context. |
| `visibility`            |          | *Visibility*            | Visibility of the constructor and move assignment operator. See possible [visibilities](#visibilities). Default is `public`. |
| `noexcept`              |          | *boolean*               | Determine whether the constructor and copy assignment operator will have the `noexcept` modifier or not. Inferred from constructor context and `definition` option. |
| `custom`                |          | *boolean*               | Determine whether the constructor will have a custom implementation or not. Default is `false`. |
| `customOperator`        |          | *boolean*               | Determine whether the move assignment operator will have a custom implementation or not. Inferred from `custom` option. |

### InitializedProperty JSON options

| Name               | Required | Type      | Description |
|--------------------|----------|-----------|-------------|
| `name`             | &#x2705; | *string*  | Name of the property that will be initialized. Must match the property `name` option. |
| `value`            |          | *string*  | Value to which the property member variable will be initialized. For example, this value can be as simple as `123` or a bit more complicated like `std::move(@)` where `@` is the `valuePlaceholder` option. The placeholder will implicitly be replaced with the constructor context specific value (for parameterless constructors it will use the property `defaultValue` option, for copy constructor it will use the member variable name and so on). Inferred from constructor context and property `type` option. |
| `valuePlaceholder` |          | *string*  | Replace value placeholder used in `value` option. Default is `@`. |
| `initializeInBody` |          | *boolean* | Determine whether the member variable will be initialized in the constructor body or not. Default is `false`. |

### Destructor JSON options

### Naming conventions

| Naming convention key   | Example |
|-------------------------|---------|
| `lowerCase`             | `naming_convention_example` |
| `upperCase`             | `NAMING_CONVENTION_EXAMPLE` |
| `camelBack`             | `namingConventionExample` |
| `camelCase`             | `NamingConventionExample` |
| `camelSnakeBack`        | `naming_Convention_Example` |
| `camelSnakeCase`        | `Naming_Convention_Example` |
| `leadingUpperSnakeCase` | `Naming_convention_example` |

### Languages

| Language key | Language |
|--------------|----------|
| `cPlusPlus`  | *C++* |
| `c`          | *C* |
| `objectiveC` | *Objective-C* |
| `swift`      | *Swift* |
| `javaScript` | *JavaScript* |

### Method modes

| Mode key      | Description |
|---------------|-------------|
| `regular`     | A plain method. |
| `virtual`     | A virtual method. |
| `pureVirtual` | A pure virtual method. |
| `override`    | A virtual method with an override specifier. |

### Visibilities

| Visibility key | Description |
|----------------|-------------|
| `public`       | A public item. |
| `protected`    | A protected item. |
| `private`      | A private item. |

### Constructor definitions

| Definition key | Description |
|----------------|-------------|
| `defined`      | The constructor will have an explicit body. |
| `default`      | The constructor is defined as default in the implementation file. |
| `deleted`      | The constructor is deleted in the header file. |
| `omitted`      | The constructor is omitted altogether. |

### Destructor definitions

| Definition key | Description |
|----------------|-------------|
| `defined`      | The destructor will have an explicit body. |
| `default`      | The destructor is defined as default in the implementation file. |
| `omitted`      | The destructor is omitted altogether. |

### Valid types

| Type key | Description |
|----------|-------------|
| `int8`   | An 8-bit signed integer. Maps to `std::int8_t` C++ type. A `number` type in JavaScript. |
| `int16`  | A 16-bit signed integer. Maps to `std::int16_t` C++ type. A `number` type in JavaScript. |
| `int32`  | A 32-bit signed integer. Maps to `std::int32_t` C++ type. A `number` type in JavaScript. |
| `int64`  | A 64-bit signed integer. Maps to `std::int64_t` C++ type. A `number` type in JavaScript. |
| `uint8`  | An 8-bit unsigned integer. Maps to `std::uint8_t` C++ type. A `number` type in JavaScript. |
| `uint16` | A 16-bit unsigned integer. Maps to `std::uint16_t` C++ type. A `number` type in JavaScript. |
| `uint32` | A 32-bit unsigned integer. Maps to `std::uint32_t` C++ type. A `number` type in JavaScript. |
| `uint64` | A 64-bit unsigned integer. Maps to `std::uint64_t` C++ type. A `number` type in JavaScript. |
| `float`  | A 32-bit IEEE 754 floating point number. Maps to `float` C++ type. A `number` type in JavaScript. |
| `double` | A 64-bit IEEE 754 floating point number. Maps to `double` C++ type. A `number` type in JavaScript. |