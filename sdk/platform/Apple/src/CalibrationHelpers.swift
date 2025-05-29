extension CalibrationHelpers {
    public static func calibrationMatrix(withPoseInWorld inWorld: Pose,
                                         andPoseInDomain inDomain: Pose,
                                         onlyRotateAroundY: Bool) -> Matrix4x4 {
        return __calibrationMatrixWithPose(inWorld:inWorld, andInDomain:inDomain, onlyRotateAroundY:onlyRotateAroundY)
    }

    public static func calibrationMatrix2(withPoseInWorld inWorld: Pose,
                                         andPoseInDomain inDomain: Pose,
                                         onlyRotateAroundY: Bool) -> Matrix4x4 {
        return __calibrationMatrix2WithPose(inWorld:inWorld, andInDomain:inDomain, onlyRotateAroundY:onlyRotateAroundY)
    }
}
