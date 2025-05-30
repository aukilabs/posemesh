posemeshModule.CalibrationHelpers = null;

__internalPosemeshAPI.builderFunctions.push(function() {
    __internalPosemesh.CalibrationHelpers.getCalibrationMatrix = function(domain, observed, onlyRotateAroundY) {
        return __internalPosemesh.CalibrationHelpers.__getCalibrationMatrix(domain, observed, onlyRotateAroundY);
    };

    posemeshModule.CalibrationHelpers = __internalPosemesh.CalibrationHelpers;
});
