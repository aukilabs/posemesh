# Posemesh SDK

_A first subset of the Posemesh SDK code will be published here soon._

The Posemesh SDK is the successor of the [ConjureKit SDK](https://conjurekit.dev), rewritten and designed from the ground up to be **open-source, cross-platform and extensible**.

The purpose of the SDK is to allow a device to exchange spatial data and compute tasks with other devices, ultimately to communicate about a shared coordinate system.

Any application, device or server interacting with the posemesh will use the Posemesh SDK to communicate with other devices in a standardised way.

## SDK Architecture
For the posemesh to work one a wide range of devices and software stacks, cross-platform support is a big part of the architecture. The majority of code is written in C++, and cross-compiled with bindings to other languages. This approach avoids code duplication and makes sure the SDK works the same regardless of platform or programming language.

### Modules

**Modules** are groups of related SDK functionality, similar to modules in the ConjureKit SDK.

- Posemesh Module (”libposemesh”)
- Feature Modules


The **Posemesh module** contains all the essential parts needed for interacting with the posemesh.

Other **feature modules** enable different types of functionality, similar to how modules are used in the ConjureKit SDK already.

### Layers

**Layers** are for supporting multiple platforms.

Each module has a “**core layer**” and multiple “**platform layers**”. The purpose of this structure is to write as much logic as possible in one language, and then cross-compiling that into each platform. This makes it easier to maintain and develop the SDK, and also to add more platforms as needed in the future without re-implementing the entire SDK code.

All modules should as much as possible follow the same structure for multi-platform support. Consistency helps reusing tools and CI pipelines, and makes it easier to get familiar with all parts of the SDK. Certain parts of the SDK may be developed with a different language or less platform-independently, if the C++ approach doesn't fit.
