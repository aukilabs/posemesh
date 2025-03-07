    /// --- END RUST JS WRAPPER --- ///

    for (const regClsFunc of regClsFuncs) {
        regClsFunc();
    }
    if (typeof source === 'object' && typeof source.then === 'function') {
        if ('instantiateStreaming' in WebAssembly) {
            __wbg_set_wasm((await WebAssembly.instantiateStreaming(source, wasmImports)).instance.exports);
            return wasmImports["./PosemeshBase_bg.js"];
        } else {
            source = await source;
        }
    }
    if (source instanceof Response) {
        source = await source.arrayBuffer();
    }
    if (!(source instanceof ArrayBuffer || source instanceof Uint8Array || (typeof Buffer !== 'undefined' && Buffer.isBuffer(source)))) {
        throw new Error('Invalid \'source\' buffer type.');
    }
    __wbg_set_wasm((await WebAssembly.instantiate(source, wasmImports)).instance.exports);
    return wasmImports["./PosemeshBase_bg.js"];
}
