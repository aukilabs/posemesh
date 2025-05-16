posemeshModule.CalibrationHelpers = null;

__internalPosemeshAPI.builderFunctions.push(function() {
    __internalPosemesh.CalibrationHelpers.getCalibrationMatrix = function(poseInDomain, observedPose, onlyRotateAroundY) {
        return __internalPosemesh.CalibrationHelpers.__getCalibrationMatrix(poseInDomain, observedPose, onlyRotateAroundY);
    };

    posemeshModule.CalibrationHelpers = __internalPosemesh.CalibrationHelpers;
});
