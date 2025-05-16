extension CalibrationHelpers {
    public static func calibrationMatrix(withPoseInDomain poseInDomain: Pose,
                                         andObservedPose observedPose: Pose,
                                         onlyRotateAroundY: Bool) -> Matrix4x4 {
        return __calibrationMatrixWithPose(inDomain:poseInDomain, andObservedPose:observedPose, onlyRotateAroundY:onlyRotateAroundY)
    }
}
