extension PoseEstimation {
     public static func solvePnP(forObjectPoints objectPoints: [Vector3], 
                         imagePoints: [Vector2], 
                         cameraMatrix: Matrix3x3, 
                         outR: inout Matrix3x3, 
                         outT: inout Vector3) -> Bool  {
                             solvePnP(forObjectPoints:objectPoints, andImagePoints:imagePoints, andCameraMatrix:cameraMatrix, withOutR:outR, andOutT:outT);
                         }
}
