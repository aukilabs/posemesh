# Posemesh Networking

The Networking module is designed to simplify the process of starting a libp2p node, enabling seamless peer-to-peer (p2p) communication within the Posemesh network. This module allows developers to integrate decentralized networking capabilities into their projects by providing an easy-to-use interface for connecting to the Posemesh network. With this module, users can join the network, discover peers, and exchange messages in a decentralized, scalable, and resilient manner.

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

### Browser
```
wasm-pack build --target web

# web app
npm install path_to_posemesh_repo/networking/pkg
```
```javascript
import init, {Networking} from 'networking';

function App() {
  const [bootstrapNodes, setBootstrapNodes] = useState('');
  const [libp2p, setLibp2p] = useState<Networking | null>(null);

  const connect = async () => {
    await init();
    const nt = new Networking(bootstrapNodes.split(","), [], "PRIVATE_KEY", false, "NODE NAME");
    setLibp2p(nt);

    // nt.send_message() for broadcasting message to all connected nodes
    // nt.poll_messages() for loading messages from all connected nodes
    // nt.nodes() for loading all nodes in posemesh network
  }

  return (
    <div className="App">
      <input type="text" value={bootstrapNodes} onChange={(e) => setBootstrapNodes(e.target.value)} />
      <button onClick={() => connect()}>Connect</button>
    </div>
  );
}
```

## Supported platforms and architectures

Below is depicted a table of platforms and architectures for which the library can be built. Intuitively, columns represent platforms and rows represent architectures.

|       | macOS | Mac-Catalyst | iOS | iOS-Simulator |
|-------|-------|--------------|-----|---------------|
| AMD64 | Yes   | Yes          | No  | Yes           |
| ARM64 | Yes   | Yes          | Yes | Yes           |

Note that building for `macOS`, `Mac-Catalyst`, `iOS` and `iOS-Simulator` can only be done on a machine that is running macOS.

## Other scripts

There are also other scripts which can aid in the development and deployment process:

- `Build-Apple.ps1` script builds all Apple platform and architecture combinations. It takes one parameter for build type which can be `Debug`, `Release` (default) or `Both`. Similarly to `Build-Library.ps1`, flag `-InstallNecessaryRustToolchainsAndTargets` can be specified to allow the underlying script calls to install the necessary Rust toolchains and targets if missing.
