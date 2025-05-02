# Posemesh Domain Browser Example

This is a browser example for the domain package. It demonstrates how to use the domain package in a browser environment.

## Prerequisites

- Node.js (v14 or later)
- npm (v6 or later)
- Rust (latest stable)
- wasm-pack (latest)

### Installing Protocol Buffers Tools

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

## Setup

1. Build the WASM module:
```bash
cd core
wasm-pack build --target web --out-dir ./examples/browser/pkg --out-name posemesh-domain --release domain
```

For macos, if you are running into `"No available targets are compatible with triple "wasm32-unknown-unknown"`, you need to use another clang compiler

```
brew install llvm
export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
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

- File upload with drag-and-drop support
- Upload status tracking
- WebRTC-based peer-to-peer communication
- Task monitoring and status updates
- File download capabilities

To modify the protobuf definitions, edit the .proto files in the `protobuf` directory and regenerate the JavaScript files using the commands above. 
