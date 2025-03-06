#import <Posemesh/PoseEstimation.h>

#include <Posemesh/PoseEstimation.hpp>

@implementation PSMPoseEstimation

+ (BOOL)solvePnPForObjectPoints:(NSArray<PSMVector3*>*)objectPoints andImagePoints:(NSArray<PSMVector2*>*)imagePoints andCameraMatrix:(PSMMatrix3x3*)cameraMatrix withOutR:(PSMMatrix3x3*)outR andOutT:(PSMVector3*)outT;
{
    NSAssert(objectPoints, @"objectPoints is null");
    NSAssert([objectPoints count] == 4, @"objectPoints array count is not 4");
    NSAssert(imagePoints, @"imagePoints is null");
    NSAssert([imagePoints count] == 4, @"imagePoints array count is not 4");
    NSAssert(cameraMatrix, @"cameraMatrix is null");
    NSAssert(outR, @"outR is null");
    NSAssert(outT, @"outT is null");

    psm::Vector3 objectPointsRaw[4];
    for (int i = 0; i < 4; ++i) {
        objectPointsRaw[i] = *static_cast<const psm::Vector3*>([objectPoints[i] nativeVector3]);
    }

    psm::Vector2 imagePointsRaw[4];
    for (int i = 0; i < 4; ++i) {
        imagePointsRaw[i] = *static_cast<const psm::Vector2*>([imagePoints[i] nativeVector2]);
    }

    const auto& cameraMatrixRaw = *static_cast<const psm::Matrix3x3*>([cameraMatrix nativeMatrix3x3]);
    auto& outRRaw = *static_cast<psm::Matrix3x3*>([outR nativeMatrix3x3]);
    auto& outTRaw = *static_cast<psm::Vector3*>([outT nativeVector3]);

    return psm::PoseEstimation::solvePnP(objectPointsRaw, imagePointsRaw, cameraMatrixRaw, outRRaw, outTRaw) ? YES : NO;
}

@end
