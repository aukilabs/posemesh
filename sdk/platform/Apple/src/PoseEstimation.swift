// when included in CMakeLists.txt produces error: cannot find type 'PoseEstimation' in scope
extension PoseEstimation {
    public static var solvePnP: Bool {
        get {
            // TODO: How to pass params for this?
            return __solvePnP()
        }
    }
}
