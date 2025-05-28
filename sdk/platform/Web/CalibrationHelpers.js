posemeshModule.CalibrationHelpers = null;

__internalPosemeshAPI.builderFunctions.push(function() {
    __internalPosemesh.CalibrationHelpers.getCalibrationMatrix = function(poseInWorld, poseInDomain, onlyRotateAroundY) {
        return __internalPosemesh.CalibrationHelpers.__getCalibrationMatrix(poseInWorld, poseInDomain, onlyRotateAroundY);
    };

    posemeshModule.CalibrationHelpers = __internalPosemesh.CalibrationHelpers;
});
