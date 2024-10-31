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
});
