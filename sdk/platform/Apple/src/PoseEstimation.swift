extension PoseEstimation {
    public static func solvePnP(forLandmarks landmarks: [Landmark], 
                                landmarkObservations: [LandmarkObservation], 
                                cameraMatrix: Matrix3x3, 
                                method: SolvePnpMethod) -> Pose {
        return __solvePnP(for:landmarks, andLandmarkObservations:landmarkObservations, andCameraMatrix:cameraMatrix, with:method);
    }

    public static func solvePnPCameraPose(forLandmarks landmarks: [Landmark], 
                                landmarkObservations: [LandmarkObservation], 
                                cameraMatrix: Matrix3x3, 
                                method: SolvePnpMethod) -> Pose {
        return __solvePnPCameraPose(for:landmarks, andLandmarkObservations:landmarkObservations, andCameraMatrix:cameraMatrix, with:method);
    }
}
