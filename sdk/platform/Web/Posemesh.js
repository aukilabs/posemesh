__internalPosemeshAPI.builderFunctions.push(function() {
    Posemesh.prototype.sendMessage = function(message, peerId, protocol) {
        return __internalPosemeshNetworking.posemeshNetworkingContextSendMessage(this.__context, message, peerId, protocol, 0);
    };

    Posemesh.prototype.sendString = function(string, appendTerminatingNullCharacter, peerId, protocol) {
        let message = new TextEncoder("utf-8").encode(string);
        if (appendTerminatingNullCharacter) {
            let newMessage = new Uint8Array(message.length + 1);
            newMessage.set(message, 0);
            newMessage.set(0, message.length);
            message = newMessage;
        }
        return __internalPosemeshNetworking.posemeshNetworkingContextSendMessage(this.__context, message, peerId, protocol, 0);
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
                // if (!__internalPosemeshAPI.verifyNetworkingCommitId()) {
                //     reject(new Error('Posemesh Networking WebAssembly file version does not match the Posemesh JavaScript file version.'));
                //     return;
                // }
                let mainPromise = mainWasmPath ? __internalPosemesh({
                    locateFile: () => mainWasmPath,
                }) : __internalPosemesh();
                mainPromise.then(mainModule => {
                    if (!__internalPosemeshAPI.verifySDKCommitId(mainModule)) {
                        reject(new Error('Posemesh SDK WebAssembly file version does not match the Posemesh JavaScript file version.'));
                        return;
                    }
                    Posemesh = mainModule.Posemesh;
                    Posemesh.__mainModule = mainModule;
                    Posemesh.Config = mainModule.Config;
                    Posemesh.Vector2f = mainModule.Vector2f;
                    Posemesh.Vector2fArray = mainModule.Vector2fArray;
                    Posemesh.Vector3f = mainModule.Vector3f;
                    Posemesh.Vector3fArray = mainModule.Vector3fArray;
                    Posemesh.Vector4f = mainModule.Vector4f;
                    Posemesh.Matrix3x3f = mainModule.Matrix3x3f;
                    Posemesh.Matrix4x4f = mainModule.Matrix4x4f;
                    Posemesh.VectorUint8 = mainModule.VectorUint8;
                    Posemesh.VectorString = mainModule.VectorString;
                    Posemesh.PoseEstimation = mainModule.PoseEstimation;
                    Posemesh.QRDetection = mainModule.QRDetection;
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
