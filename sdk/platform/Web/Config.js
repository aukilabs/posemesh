posemeshModule.Config = null;

__internalPosemeshAPI.builderFunctions.push(function(context) {
    __internalPosemesh.Config.prototype.getBootstraps = function() {
        let bootstraps = this.__getBootstraps();
        try {
            return __internalPosemeshAPI.fromVectorString(bootstraps);
        } finally {
            bootstraps.delete();
        }
    };

    __internalPosemesh.Config.prototype.setBootstraps = function(bootstraps) {
        let vectorString = __internalPosemeshAPI.toVectorString(bootstraps);
        try {
            return this.__setBootstraps(vectorString);
        } finally {
            vectorString.delete();
        }
    };

    Object.defineProperty(__internalPosemesh.Config.prototype, 'bootstraps', {
        get: __internalPosemesh.Config.prototype.getBootstraps,
        set: function(bootstraps) {
            this.setBootstraps(bootstraps);
        },
        enumerable: true,
        configurable: false
    });

    __internalPosemesh.Config.prototype.getRelays = function() {
        let relays = this.__getRelays();
        try {
            return __internalPosemeshAPI.fromVectorString(relays);
        } finally {
            relays.delete();
        }
    };

    __internalPosemesh.Config.prototype.setRelays = function(relays) {
        let vectorString = __internalPosemeshAPI.toVectorString(relays);
        try {
            return this.__setRelays(vectorString);
        } finally {
            vectorString.delete();
        }
    };

    Object.defineProperty(__internalPosemesh.Config.prototype, 'relays', {
        get: __internalPosemesh.Config.prototype.getRelays,
        set: function(relays) {
            this.setRelays(relays);
        },
        enumerable: true,
        configurable: false
    });

    __internalPosemesh.Config.prototype.getPrivateKey = function() {
        let privateKey = this.__getPrivateKey();
        try {
            return __internalPosemeshAPI.fromVectorUint8(privateKey);
        } finally {
            privateKey.delete();
        }
    };

    __internalPosemesh.Config.prototype.setPrivateKey = function(privateKey) {
        let vectorUint8 = __internalPosemeshAPI.toVectorUint8(privateKey);
        try {
            this.__setPrivateKey(vectorUint8);
        } finally {
            vectorUint8.delete();
        }
    };

    Object.defineProperty(__internalPosemesh.Config.prototype, 'privateKey', {
        get: __internalPosemesh.Config.prototype.getPrivateKey,
        set: __internalPosemesh.Config.prototype.setPrivateKey,
        enumerable: true,
        configurable: false
    });

    Object.defineProperty(__internalPosemesh.Config.prototype, 'name', {
        get: __internalPosemesh.Config.prototype.getName,
        set: __internalPosemesh.Config.prototype.setName,
        enumerable: true,
        configurable: false
    });

    posemeshModule.Config = __internalPosemesh.Config;
});
