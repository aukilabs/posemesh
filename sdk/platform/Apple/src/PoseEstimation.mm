/* This code is automatically generated from PoseEstimation.json interface. Do not modify it manually as it will be overwritten! */

#import <Posemesh/PoseEstimation.h>

#include <Posemesh/PoseEstimation.hpp>

@implementation PSMPoseEstimation

// + (BOOL)solvePnP
// {
//     NSAssert(m_poseEstimation.get() != nullptr, @"m_poseEstimation is null");
//     return m_poseEstimation.get()->getSolvePnP() ? YES : NO;
// }
+ (BOOL)solvePnPForObjectPoints:(NSArray *)objectPoints imagePoints:(NSArray *)imagePoints cameraMatrix:(PSMMatrix3x3f *)cameraMatrix outR:(PSMMatrix3x3f *)outR outT:(PSMVector3f *)outT {
    // NSAssert(m_poseEstimation.get() != nullptr, @"m_poseEstimation is null");

    // PSMVector3f *objectPoints -> float[]
    float oPoints[12];
    oPoints[0] = [objectPoints[0] x];
    oPoints[1] = [objectPoints[0] y];
    oPoints[2] = [objectPoints[0] z];
    
    oPoints[3] = [objectPoints[1] x];
    oPoints[4] = [objectPoints[1] y];
    oPoints[5] = [objectPoints[1] z];
    
    oPoints[6] = [objectPoints[2] x];
    oPoints[7] = [objectPoints[2] y];
    oPoints[8] = [objectPoints[2] z];
    
    oPoints[9] = [objectPoints[3] x];
    oPoints[10] = [objectPoints[3] y];
    oPoints[11] = [objectPoints[3] z];
    
    for (int i = 0; i < 12; i += 3) {
        NSLog(@"oPoint = %f, %f, %f", oPoints[i], oPoints[i + 1], oPoints[i + 2]);
    }

    // PSMVector2f *imagePoints -> float[]
    float iPoints[8];
    iPoints[0] = [imagePoints[0] x];
    iPoints[1] = [imagePoints[0] y];

    iPoints[2] = [imagePoints[1] x];
    iPoints[3] = [imagePoints[1] y];

    iPoints[4] = [imagePoints[2] x];
    iPoints[5] = [imagePoints[2] y];

    iPoints[6] = [imagePoints[3] x];
    iPoints[7] = [imagePoints[3] y];

    for (int i = 0; i < 8; i += 2) {
        NSLog(@"iPoint =  %f, %f", iPoints[i], iPoints[i + 1]);
    }

    // PSMMatrix3x3f *cameraMatrix -> float *
    float cMatrix[9];
    cMatrix[0] = [cameraMatrix m00];
    cMatrix[1] = [cameraMatrix m01];
    cMatrix[2] = [cameraMatrix m02];
    cMatrix[3] = [cameraMatrix m10];
    cMatrix[4] = [cameraMatrix m11];
    cMatrix[5] = [cameraMatrix m12];
    cMatrix[6] = [cameraMatrix m20];
    cMatrix[7] = [cameraMatrix m21];
    cMatrix[8] = [cameraMatrix m22];

    for (int i = 0; i < 9; i += 3) {
        NSLog(@"cm =  %f, %f, %f", cMatrix[i], cMatrix[i + 1], cMatrix[i + 2]);
    }

    psm::Vector3f translation;
    psm::Matrix3x3f rotation;
    BOOL solveResult = psm::PoseEstimation::solvePnP(oPoints, iPoints, cMatrix, &rotation, &translation);
    
    // use the temp C++ object to create the PSMVector3f with initWithNativeMatrix3x3f
    [outT setX:translation.getX()];
    [outT setY:translation.getY()];
    [outT setZ:translation.getZ()];

    [outR setM00:rotation.getM00()];
    [outR setM01:rotation.getM01()];
    [outR setM02:rotation.getM02()];
    [outR setM10:rotation.getM10()];
    [outR setM11:rotation.getM11()];
    [outR setM12:rotation.getM12()];
    [outR setM20:rotation.getM20()];
    [outR setM21:rotation.getM21()];
    [outR setM22:rotation.getM22()];
    
    return solveResult;
}


@end
