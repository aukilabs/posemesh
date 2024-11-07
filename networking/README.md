# Posemesh Networking

Posemesh Networking is a Rust library that implements all of the underlying network code for efficient and optimized communication between nodes in the Posemesh network. The module is designed to simplify the process of running a peer-to-peer (P2P) Posemesh node, allowing easy and seamless P2P communication within the Posemesh network. With this module, nodes can join the network, discover peers, and exchange messages in a decentralized, scalable, and resilient manner.

## Building

First, ensure that you have [Rust](https://www.rust-lang.org/tools/install) installed on your machine. Building the library has been delegated to the `Build-Library.ps1` PowerShell script. For that reason you also need to have [PowerShell](https://learn.microsoft.com/en-us/powershell/scripting/install/installing-powershell) installed on your machine.

To build the library simply run the following command in PowerShell in the current directory:

```
./scripts/Build-Library.ps1 <Platform> <Architecture> <BuildType>
```

Replace the `<Platform>` and `<Architecture>` with the platform and architecture combination for which you want to build the library. Refer to the [supported platforms and architectures](#supported-platforms-and-architectures) table below. As of `<BuildType>`, you should replace it with either `Debug` or `Release`, depending on which configuration you wish to produce.

Some platform and architecture combinations may require specific Rust toolchains and targets to be installed. The `Build-Library.ps1` script can install them automatically, however you first need to allow it to do so. You can do that by also specifying the `-InstallNecessaryRustToolchainsAndTargets` flag. The command will now look as follows:

```
./scripts/Build-Library.ps1 <Platform> <Architecture> <BuildType> -InstallNecessaryRustToolchainsAndTargets
```

The build results end up in a newly-created `target` directory. They include a static library alongside some C++ header files with exported types and function declarations.

## Supported platforms and architectures

Below is depicted a table of platforms and architectures for which the library can be built. Intuitively, columns represent platforms and rows represent architectures.

|        | macOS | Mac-Catalyst | iOS | iOS-Simulator | Web |
|--------|-------|--------------|-----|---------------|-----|
| AMD64  | Yes   | Yes          | No  | Yes           | No  |
| ARM64  | Yes   | Yes          | Yes | Yes           | No  |
| WASM32 | No    | No           | No  | No            | Yes |

Note that building for `macOS`, `Mac-Catalyst`, `iOS` and `iOS-Simulator` can only be done on a machine that is running macOS.

## Other scripts

There are also other scripts which can aid in the development and deployment process:

- `Build-Apple.ps1` script builds all Apple platform and architecture combinations. It takes one parameter for build type which can be `Debug`, `Release` (default) or `Both`. Similarly to `Build-Library.ps1`, flag `-InstallNecessaryRustToolchainsAndTargets` can be specified to allow the underlying script calls to install the necessary Rust toolchains and targets if missing.

- `. ./scripts/pybindgen.sh` generates python wrapper. You need to have python3 installed

# Notes

pyo3 is using [0.20.0](https://pyo3.rs/v0.20.0/) version because pyo3-asyncio requires.
