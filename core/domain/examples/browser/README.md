# Posemesh Browser Example

This is a browser-based example of using Posemesh for file uploads and downloads with WebRTC support.

## Prerequisites

- Node.js (v14 or later)
- npm
- protoc (Protocol Buffers Compiler)
- protoc-gen-js (Protocol Buffers JavaScript Plugin)

## Setup

1. Install dependencies:
```bash
npm install
```

2. Generate JavaScript Protobuf files:

First, install the protobuf compiler and JavaScript plugin:

```bash
# macOS (using Homebrew)
brew install protobuf
npm install -g protoc-gen-js

# Linux
sudo apt-get install protobuf-compiler
npm install -g protoc-gen-js

# Windows (using chocolatey)
choco install protoc
npm install -g protoc-gen-js
```

Then generate the JavaScript files from your .proto files:

```bash
# From the browser example directory
protoc \
--plugin=protoc-gen-js=$(which protoc-gen-js) \
--proto_path=../../../protobuf \
--js_out=import_style=es6,binary:./protobuf ../../../protobuf/*.proto
```

This will generate the following files in the `protobuf` directory:
- `task_pb.js`
- `domain_data_pb.js`

## Building

1. Build the JavaScript bundle:
```bash
npm run build
```

2. For development with auto-rebuild:
```bash
npm run watch
```

## Running

Start the development server:
```bash
npm run serve
```

The application will be available at http://localhost:3000

## Project Structure

```
browser/
├── build.js          # esbuild configuration
├── index.html        # Main HTML file
├── main.js          # Main JavaScript entry point
├── package.json     # Project dependencies and scripts
├── protobuf/        # Generated protobuf JavaScript files
└── pkg/             # WebAssembly package directory
```

## Features

- File upload with drag-and-drop support
- Upload status tracking
- WebRTC-based peer-to-peer communication
- Task monitoring and status updates
- File download capabilities

## Development

The project uses:
- esbuild for bundling
- Tailwind CSS for styling
- Protocol Buffers for data serialization
- WebAssembly for core functionality

To modify the protobuf definitions, edit the .proto files in the `proto` directory and regenerate the JavaScript files using the commands above. 
