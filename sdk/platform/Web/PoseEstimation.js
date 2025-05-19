posemeshModule.PoseEstimation = null;

__internalPosemeshAPI.builderFunctions.push(function() {
    __internalPosemesh.PoseEstimation.solvePnP = function(landmarks, landmarkObservations, cameraMatrix, method) {
        let landmarksVector = undefined, landmarkObservationsVector = undefined;
        try {
            landmarksVector = __internalPosemeshAPI.toVectorLandmark(landmarks, false);
            landmarkObservationsVector = __internalPosemeshAPI.toVectorLandmarkObservation(landmarkObservations, false);
            return __internalPosemesh.PoseEstimation.__solvePnP(landmarksVector, landmarkObservationsVector, cameraMatrix, method);
        } finally {
            if (landmarkObservationsVector) {
                landmarkObservationsVector.delete();
            }
            if (landmarksVector) {
                landmarksVector.delete();
            }
        }
    };

    posemeshModule.PoseEstimation = __internalPosemesh.PoseEstimation;
});
