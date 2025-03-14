posemeshModule.Landmark = null;

__internalPosemeshAPI.builderFunctions.push(function() {
    Object.defineProperty(__internalPosemesh.Landmark.prototype, 'type', {
        get: __internalPosemesh.Landmark.prototype.__getType,
        set: __internalPosemesh.Landmark.prototype.__setType,
        enumerable: true,
        configurable: false
    });

    Object.defineProperty(__internalPosemesh.Landmark.prototype, 'id', {
        get: __internalPosemesh.Landmark.prototype.__getId,
        set: __internalPosemesh.Landmark.prototype.__setId,
        enumerable: true,
        configurable: false
    });

    Object.defineProperty(__internalPosemesh.Landmark.prototype, 'position', {
        get: __internalPosemesh.Landmark.prototype.__getPosition,
        set: __internalPosemesh.Landmark.prototype.__setPosition,
        enumerable: true,
        configurable: false
    });

    posemeshModule.Landmark = __internalPosemesh.Landmark;
});
