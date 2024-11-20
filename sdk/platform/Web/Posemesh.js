__internalPosemeshAPI.builderFunctions.push(function() {
    Posemesh.prototype.sendMessage = function(message, peerId, protocol) {
        return __internalPosemeshNetworking.posemeshNetworkingContextSendMessage(this.__context, message, peerId, protocol);
    };

    Posemesh.prototype.sendString = function(string, appendTerminatingNullCharacter, peerId, protocol) {
        let message = new TextEncoder("utf-8").encode(string);
        if (appendTerminatingNullCharacter) {
            let newMessage = new Uint8Array(message.length + 1);
            newMessage.set(message, 0);
            newMessage.set(0, message.length);
            message = newMessage;
        }
        return __internalPosemeshNetworking.posemeshNetworkingContextSendMessage(this.__context, message, peerId, protocol);
    };
});

var Posemesh = {
    mainWasmPath: null,
    networkingWasmPath: null,
    initialize: () => {
        let mainWasmPath = Posemesh.mainWasmPath;
        let networkingWasmPath = Posemesh.networkingWasmPath;
        Posemesh = {};
        return new Promise((resolve, reject) => {
            if (mainWasmPath !== null && typeof mainWasmPath !== 'string') {
                reject(new Error('Posemesh.mainWasmPath should be a string containing the path of the Posemesh.wasm file in case it was renamed and/or relocated or null to use the default name and path.'));
                return;
            }
            if (networkingWasmPath !== null && typeof networkingWasmPath !== 'string') {
                reject(new Error('Posemesh.networkingWasmPath should be a string containing the path of the PosemeshNetworking.wasm file in case it was renamed and/or relocated or null to use the default name and path.'));
                return;
            }
            let networkingPromise = networkingWasmPath ? __internalPosemeshNetworking(networkingWasmPath) : __internalPosemeshNetworking();
            networkingPromise.then(() => {
                let mainPromise = mainWasmPath ? __internalPosemesh({
                    locateFile: () => mainWasmPath,
                }) : __internalPosemesh();
                mainPromise.then(mainModule => {
                    Posemesh = mainModule.Posemesh;
                    Posemesh.__mainModule = mainModule;
                    Posemesh.Config = mainModule.Config;
                    for (let builderFunction of __internalPosemeshAPI.builderFunctions) {
                        builderFunction();
                    }
                    resolve(Posemesh);
                }).catch(error => {
                    if (!error) {
                        reject(new Error('Failed to initialize the main Posemesh module.'));
                        return;
                    }
                    if (typeof error !== "string")
                        error = error.toString();
                    reject(new Error('Failed to initialize the main Posemesh module: ' + error));
                })
            }).catch(error => {
                if (!error) {
                    reject(new Error('Failed to initialize the networking Posemesh module.'));
                    return;
                }
                if (typeof error !== "string")
                    error = error.toString();
                reject(new Error('Failed to initialize the networking Posemesh module: ' + error));
            });
        });
    },
};
