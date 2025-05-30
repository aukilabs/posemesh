extension CalibrationHelpers {
    public static func calibrationMatrix(withDomainPose domain: Pose,
                                         andObservedPose observed: Pose,
                                         onlyRotateAroundY: Bool) -> Matrix4x4 {
        return __calibrationMatrix(withDomainPose:domain, andObservedPose:observed, onlyRotateAroundY:onlyRotateAroundY)
    }
}
