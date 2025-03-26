# GenTool

## JSON documentation

### Root JSON options

| Name                         | Required | Type                       | Description |
|------------------------------|----------|----------------------------|-------------|
| `name`                       | &#x2705; | *string*                   | Name of the class in `Camel_Snake_Case` naming convention. The different [naming conventions](#naming-conventions) are automatically derived from this one, however they can be manually specified with options such as `name.style.lowerCase`, `name.style.upperCase` and such. Furthermore, different [language](#languages) binding names are taken from the generated naming conventions, but they can also be manually specified with options such as `name.lang.cPlusPlus`, `name.lang.c` and such. |
| `static`                     |          | *boolean*                  | Determine whether the class will be static or not. Default is `false`. |
| `final`                      |          | *boolean*                  | Determine whether the class will be final or not. Inferred from `static` option. |
| `aliases`                    |          | *Alias[]*                  | Alternative names of the class. No aliases by default. See [definition](#alias-json-options). |
| `headerGuardName`            |          | *string*                   | Name of the class in `UPPER_CASE` naming convention used for C++ and C header guards. Derived from `name` option. |
| `properties`                 |          | *Property[]*               | Properties of the class. No properties by default. See [definition](#property-json-options). |
| `copyable`                   |          | *boolean*                  | Determine whether the class is copyable or not. Inferred from `static` option. |
| `movable`                    |          | *boolean*                  | Determine whether the class is movable or not. Inferred from `static` option. |
| `parameterlessConstructor`   |          | *ParameterlessConstructor* | Options for the parameterless constructor. See [definition](#parameterlessconstructor-json-options). |
| `copyConstructor`            |          | *CopyConstructor*          | Options for the copy constructor and operator. See [definition](#copyconstructor-json-options). |
| `moveConstructor`            |          | *MoveConstructor*          | Options for the move constructor and operator. See [definition](#moveconstructor-json-options). |
| `destructor`                 |          | *Destructor*               | Options for the destructor. See [definition](#destructor-json-options). |
| `equalityOperator`           |          | *EqualityOperator*         | Options for the equality (and inequality) operator. See [definition](#equalityoperator-json-options). |
| `hashOperator`               |          | *HashOperator*             | Options for the hash operator. See [definition](#hashoperator-json-options). |
| `toStringOperator`           |          | *ToStringOperator*         | Options for the to-string operator. See [definition](#tostringoperator-json-options). |
| `c.generateFuncAliasDefines` |          | *boolean*                  | Determine whether the C header will contain the macro defines for all class functions with the first part of the function replaced with names for all class aliases or not. Default is `true`. |

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
| `partOfIdentity`   |          | *boolean*    | Determine whether the property is considered to be a part of identity of the class instance or not. If set to `true` the property will be used in the equality, inequality and hash operators. Inferred from `static` option. |

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

| Name         | Required | Type                   | Description |
|--------------|----------|------------------------|-------------|
| `virtual`    |          | *boolean*              | Determine wether the destructor is virtual or not. Default is `false`. |
| `code`       |          | *string[]*             | Destructor code lines placed in the destructor body. Default is an empty array. |
| `definition` |          | *DestructorDefinition* | Determine which destructor definition mode to use. See [possible options](#destructor-definitions). Inferred from `code` option. |
| `visibility` |          | *Visibility*           | Visibility of the destructor. See possible [visibilities](#visibilities). Default is `public`. |
| `custom`     |          | *boolean*              | Determine whether the destructor will have a custom implementation or not. Default is `false`. |

### EqualityOperator JSON options

| Name                 | Required | Type                 | Description |
|----------------------|----------|----------------------|-------------|
| `defined`            |          | *boolean*            | Determine whether the equality and inequality operators are defined or not. Inferred from class `static` option. |
| `comparePointers`    |          | *boolean*            | Determine whether the equality and inequality operators will just compare the class instance pointer or not. Inferred from class `copyable` option. |
| `comparedProperties` |          | *ComparedProperty[]* | Compared properties in the equality and inequality operators. Inferred from class `properties` option. See [definition](#comparedproperty-json-options). |
| `custom`             |          | *boolean*            | Determine whether the equality operator will have a custom implementation or not. Default is `false`. |
| `customInequality`   |          | *boolean*            | Determine whether the inequality operator will have a custom implementation or not. Default is `false`. |

### ComparedProperty JSON options

| Name                                 | Required | Type      | Description |
|--------------------------------------|----------|-----------|-------------|
| `name`                               | &#x2705; | *string*  | Name of the property that will be compared. Must match the property `name` option. |
| `useGetter`                          |          | *boolean* | Determine whether to use the property getter method or not. Inferred from property `hasMemberVar` and `hasGetter` options. |
| `comparator`                         |          | *string*  | Expression evaluating to a boolean used to test equality of the property. For example, this comparator can be as simple as `true` or a bit more complicated like `$ == @.$` where `@` is the `comparatorClassInstancePlaceholder` option and `$` is the `comparatorPropertyPlaceholder` option. The `@` (`comparatorClassInstancePlaceholder`) placeholder will implicitly be replaced with the name of the other class instance argument name. The `$` (`comparatorPropertyPlaceholder`) placeholder will implicitly be replaced with either the named access of the property member variable or the property getter method call. Inferred from `comparatorClassInstancePlaceholder` and `comparatorPropertyPlaceholder` options as well as property `type` option. |
| `comparatorClassInstancePlaceholder` |          | *string*  | Replace comparator placeholder used in `comparator` option. Default is `@`. |
| `comparatorPropertyPlaceholder`      |          | *string*  | Replace comparator placeholder used in `comparator` option. Default is `$`. |

### HashOperator JSON options

| Name               | Required | Type               | Description |
|--------------------|----------|--------------------|-------------|
| `defined`          |          | *boolean*          | Determine whether the hash operator is defined or not. Inferred from [equality operator](#equalityoperator-json-options) `defined` option. |
| `usePointerAsHash` |          | *boolean*          | Determine whether the hash operator will just return the class instance pointer or not. Inferred from [equality operator](#equalityoperator-json-options) `comparePointers` option. |
| `hashedProperties` |          | *HashedProperty[]* | Hashed properties in the hash operator. Inferred from [equality operator](#equalityoperator-json-options) `comparedProperties` option. See [definition](#hashedproperty-json-options). |
| `custom`           |          | *boolean*          | Determine whether the hash operator will have a custom implementation or not. Inferred from [equality operator](#equalityoperator-json-options) `custom` option. |

### HashedProperty JSON options

| Name                | Required | Type      | Description |
|---------------------|----------|-----------|-------------|
| `name`              | &#x2705; | *string*  | Name of the property that will be hashed. Must match the property `name` option. |
| `useGetter`         |          | *boolean* | Determine whether to use the property getter method or not. Inferred from [compared property](#comparedproperty-json-options) `useGetter` option if possible or property `hasMemberVar` and `hasGetter` options. |
| `hasher`            |          | *string*  | Expression evaluating to a hash integer used to hash the property. For example, this hasher can be as simple as `123` or a bit more complicated like `hash<float> {}(@)` where `@` is the `hasherPlaceholder` option. The placeholder will implicitly be replaced with either the named access of the property member variable or the property getter method call. Inferred from `hasherPlaceholder` option as well as property `type` option. |
| `hasherPlaceholder` |          | *string*  | Replace hasher placeholder used in `hasher` option. Default is `@`. |

### ToStringOperator JSON options

| Name      | Required | Type      | Description |
|-----------|----------|-----------|-------------|
| `defined` |          | *boolean* | Determine whether the to-string operator is defined or not. Inferred from class `static` option. |
| `custom`  |          | *boolean* | Determine whether the to-string operator will have a custom implementation or not. Default is `false`. |

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

| Type key               | Description |
|------------------------|-------------|
| `int8`                 | An 8-bit signed integer. Maps to `std::int8_t` C++ type. A `number` type in JavaScript. |
| `int16`                | A 16-bit signed integer. Maps to `std::int16_t` C++ type. A `number` type in JavaScript. |
| `int32`                | A 32-bit signed integer. Maps to `std::int32_t` C++ type. A `number` type in JavaScript. |
| `int64`                | A 64-bit signed integer. Maps to `std::int64_t` C++ type. A `bigint` type in JavaScript. |
| `uint8`                | An 8-bit unsigned integer. Maps to `std::uint8_t` C++ type. A `number` type in JavaScript. |
| `uint16`               | A 16-bit unsigned integer. Maps to `std::uint16_t` C++ type. A `number` type in JavaScript. |
| `uint32`               | A 32-bit unsigned integer. Maps to `std::uint32_t` C++ type. A `number` type in JavaScript. |
| `uint64`               | A 64-bit unsigned integer. Maps to `std::uint64_t` C++ type. A `bigint` type in JavaScript. |
| `float`                | A 32-bit IEEE 754 floating point number. Maps to `float` C++ type. A `number` type in JavaScript. |
| `double`               | A 64-bit IEEE 754 floating point number. Maps to `double` C++ type. A `number` type in JavaScript. |
| `boolean`              | A boolean type. Maps to `bool` C++ type. A `boolean` type in JavaScript. |
| `string`               | A string type. Maps to `std::string` C++ type (getter and setter use it by value). A `string` type in JavaScript. |
| `string_ref`           | A string type. Maps to `std::string` C++ type (getter and setter use it by const-ref). A `string` type in JavaScript. |
| `string_mix`           | A string type. Maps to `std::string` C++ type (getter uses it by const-ref and setter uses it by value). A `string` type in JavaScript. |
| `ENUM:<TYPE>`          | A custom generated enum `<TYPE>` type. |
| `CLASS:<TYPE>`         | A custom generated class `<TYPE>` type. In C++ getter and setter use it by value. |
| `CLASS_REF:<TYPE>`     | A custom generated class `<TYPE>` type. In C++ getter and setter use it by const-ref. |
| `CLASS_MIX:<TYPE>`     | A custom generated class `<TYPE>` type. In C++ getter uses it by const-ref and setter uses it by value. |
| `CLASS_PTR:<TYPE>`     | A custom generated class `<TYPE>` type wrapped in a `std::shared_ptr` smart pointer. In C++ getter and setter use it by value. |
| `CLASS_PTR_REF:<TYPE>` | A custom generated class `<TYPE>` type wrapped in a `std::shared_ptr` smart pointer. In C++ getter and setter use it by const-ref. |
| `CLASS_PTR_MIX:<TYPE>` | A custom generated class `<TYPE>` type wrapped in a `std::shared_ptr` smart pointer. In C++ getter uses it by const-ref and setter uses it by value. |
