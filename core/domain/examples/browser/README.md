# Posemesh Domain Browser Example

This is a browser example for the domain package. It demonstrates how to use the domain package in a browser environment.

## Prerequisites
- Node.js (v14 or later)
- npm (v6 or later)

## Setup
1.a Install the WASM module:

Setup credentials: https://conjurekit.dev/unity/quickstart/#authenticate-towards-the-registry
```bash
npm install @aukilabs/posemesh-domain
```

Ensure your .npmrc file is correctly configured to access the Auki registry. You can find this file in either the /core/examples/browser/.npmrc directory or your home directory as ~/.npmrc.
Add the following lines to your .npmrc file:
```
//npm.dev.aukiverse.com/:_authToken="***"
@aukilabs:registry=https://npm.dev.aukiverse.com/
```
By configuring your .npmrc file as shown, you'll be able to seamlessly access and install packages from the Auki registry.

1.b Build the WASM module:
- Install Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

- Install wasm-pack
```bash
cargo install wasm-pack
cd core
```

- (Optional) Installing Protocol Buffers Tools

install 
- protoc (Protocol Buffers Compiler)
- protoc-gen-ts (Protocol Buffers TypeScript Plugin)

```bash
# macOS (using Homebrew)
brew install protobuf
npm install -g protoc-gen-ts

# Linux
sudo apt-get install protobuf-compiler
npm install -g protoc-gen-ts

# Windows (using chocolatey)
choco install protoc
npm install -g protoc-gen-ts
```

- Build rust
```bash
make bundle-domain
```
For macos, if you are running into `"No available targets are compatible with triple "wasm32-unknown-unknown"`, you need to use another clang compiler

```
brew install llvm
export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
```

- Link module
Link once
```bash
cd core/domain/pkg
npm link

cd core/domain/examples/browser
npm link @aukilabs/posemesh-domain
```

2. Install dependencies:
```bash
cd domain/examples/browser
npm install
```

3. Start the development server:
Update .env, you should copy webrtc address of domain manager and any string as app id
```bash
npm run dev
```

4. Open your browser and navigate to `http://localhost:5173`


## Development

The project uses:
- Vite for development and building
- Tailwind CSS for styling
- Protocol Buffers for data serialization
- WebAssembly for core functionality

## Building for Production

```bash
npm run build
```

This will create a production build in the `dist` directory.

## Features

- Scans upload with drag-and-drop support
- Upload status tracking
- WebRTC-based peer-to-peer communication
- Trigger refinement on selected scans
- Scans download capabilities

To modify the protobuf definitions, edit the .proto files in the `protobuf` directory and regenerate the JavaScript files using the commands above. 
