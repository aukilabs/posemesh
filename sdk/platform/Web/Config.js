__internalPosemeshAPI.builderFunctions.push(function() {
    Posemesh.Config.prototype.getBootstraps = function() {
        let bootstraps = this.__getBootstraps();
        try {
            return __internalPosemeshAPI.fromVectorString(bootstraps);
        } finally {
            bootstraps.delete();
        }
    };

    Posemesh.Config.prototype.setBootstraps = function(bootstraps) {
        let vectorString = __internalPosemeshAPI.toVectorString(bootstraps);
        try {
            return this.__setBootstraps(vectorString);
        } finally {
            vectorString.delete();
        }
    };

    Object.defineProperty(Posemesh.Config.prototype, 'bootstraps', {
        get: Posemesh.Config.prototype.getBootstraps,
        set: function(bootstraps) {
            this.setBootstraps(bootstraps);
        },
        enumerable: true,
        configurable: false
    });

    Posemesh.Config.prototype.getRelays = function() {
        let relays = this.__getRelays();
        try {
            return __internalPosemeshAPI.fromVectorString(relays);
        } finally {
            relays.delete();
        }
    };

    Posemesh.Config.prototype.setRelays = function(relays) {
        let vectorString = __internalPosemeshAPI.toVectorString(relays);
        try {
            return this.__setRelays(vectorString);
        } finally {
            vectorString.delete();
        }
    };

    Object.defineProperty(Posemesh.Config.prototype, 'relays', {
        get: Posemesh.Config.prototype.getRelays,
        set: function(relays) {
            this.setRelays(relays);
        },
        enumerable: true,
        configurable: false
    });

    Posemesh.Config.prototype.getPrivateKey = function() {
        let privateKey = this.__getPrivateKey();
        try {
            return __internalPosemeshAPI.fromVectorUint8(privateKey);
        } finally {
            privateKey.delete();
        }
    };

    Posemesh.Config.prototype.setPrivateKey = function(privateKey) {
        let vectorUint8 = __internalPosemeshAPI.toVectorUint8(privateKey);
        try {
            this.__setPrivateKey(vectorUint8);
        } finally {
            vectorUint8.delete();
        }
    };

    Object.defineProperty(Posemesh.Config.prototype, 'privateKey', {
        get: Posemesh.Config.prototype.getPrivateKey,
        set: Posemesh.Config.prototype.setPrivateKey,
        enumerable: true,
        configurable: false
    });
});
