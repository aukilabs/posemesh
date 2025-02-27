    /// --- END MAIN JS WRAPPER --- ///

    posemeshModule.initializePosemesh = async function(baseWasmSource = undefined, mainWasmSource = undefined) {
        if (typeof baseWasmSource === 'undefined') {
            baseWasmSource = './PosemeshBase.wasm';
        }
        if (typeof mainWasmSource === 'undefined') {
            mainWasmSource = './Posemesh.wasm';
        }
        if (!posemeshModuleCanInitialize) {
            throw new Error('Cannot initialize Posemesh module.');
        }
        posemeshModuleCanInitialize = false;
        try {
            __internalPosemeshBase = await __internalPosemeshBase(baseWasmSource);
        } catch (error) {
            posemeshModuleCanInitialize = true;
            if (typeof error !== 'string') {
                error = error.toString();
            }
            throw new Error('Failed to initialize Posemesh Base WebAssembly: ' + error);
        }
        if (!__internalPosemeshAPI.verifyBaseCommitId()) {
            throw new Error('Posemesh Base WebAssembly file version does not match the Posemesh JavaScript file version.');
        }
        try {
            let source = mainWasmSource;
            if (source instanceof URL) {
                source = source.href;
            }
            if (typeof source === 'string') {
                if (typeof window === 'undefined') {
                    let fsPromises = undefined;
                    try {
                        fsPromises = require('fs/promises');
                    } catch {
                        fsPromises = require('fs').promises;
                    }
                    source = await fsPromises.readFile(source);
                } else {
                    if (typeof fetch !== 'function') {
                        throw new Error('Unavailable \'fetch()\' function.');
                    }
                    source = fetch(source);
                }
            }
            let posemeshSignal = undefined;
            let posemeshSignalPromise = new Promise(function(resolve) {
                posemeshSignal = resolve;
            });
            let posemeshSignalError = undefined;
            let posemeshPromise = __internalPosemesh({
                instantiateWasm: function(imports, successCallback) {
                    (async function() {
                        if (typeof source === 'object' && typeof source.then === 'function') {
                            if ('instantiateStreaming' in WebAssembly) {
                                return (await WebAssembly.instantiateStreaming(source, imports)).instance;
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
                        return (await WebAssembly.instantiate(source, imports)).instance;
                    })().then(function(result) {
                        successCallback(result);
                        posemeshSignal();
                    }).catch(function(error) {
                        posemeshSignalError = error;
                        posemeshSignal();
                    });
                    return {};
                }
            });
            await posemeshSignalPromise;
            if (typeof posemeshSignalError !== 'undefined') {
                throw posemeshSignalError;
            }
            __internalPosemesh = await posemeshPromise;
        } catch (error) {
            if (typeof error !== 'string') {
                error = error.toString();
            }
            throw new Error('Failed to initialize Posemesh WebAssembly: ' + error);
        }
        if (!__internalPosemeshAPI.verifyMainCommitId()) {
            throw new Error('Posemesh WebAssembly file version does not match the Posemesh JavaScript file version.');
        }
        for (let builderFunction of __internalPosemeshAPI.builderFunctions) {
            builderFunction({});
        }
    };
    return posemeshModule;
}));
