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
});
