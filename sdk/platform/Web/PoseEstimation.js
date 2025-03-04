posemeshModule.PoseEstimation = null;

__internalPosemeshAPI.builderFunctions.push(function() {
    __internalPosemesh.PoseEstimation.solvePnP = function(objectPoints, imagePoints, cameraMatrix, outR, outT) {
        let objectPointsVector = undefined, imagePointsVector = undefined;
        try {
            objectPointsVector = __internalPosemeshAPI.toVectorVector3f(objectPoints, false);
            imagePointsVector = __internalPosemeshAPI.toVectorVector2f(imagePoints, false);
            return __internalPosemesh.PoseEstimation.__solvePnP(objectPointsVector, imagePointsVector, cameraMatrix, outR, outT);
        } finally {
            if (imagePointsVector) {
                imagePointsVector.delete();
            }
            if (objectPointsVector) {
                objectPointsVector.delete();
            }
        }
    };

    posemeshModule.PoseEstimation = __internalPosemesh.PoseEstimation;
});
