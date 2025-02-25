extension PoseEstimation {
     public static func solvePnP(forObjectPoints objectPoints: [Vector3f], 
                         imagePoints: [Vector2f], 
                         cameraMatrix: Matrix3x3f, 
                         outR: inout Matrix3x3f, 
                         outT: inout Vector3f) -> Bool  {
                            __solvePnP(forObjectPoints:objectPoints, imagePoints:imagePoints, cameraMatrix:cameraMatrix, outR:outR, outT:outT);
                         }
}
