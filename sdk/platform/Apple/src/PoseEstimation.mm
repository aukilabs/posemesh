#import <Posemesh/PoseEstimation.h>

#include <Posemesh/PoseEstimation.hpp>

@implementation PSMPoseEstimation

+ (BOOL)solvePnPForObjectPoints:(NSArray<PSMVector3f*>*)objectPoints andImagePoints:(NSArray<PSMVector2f*>*)imagePoints andCameraMatrix:(PSMMatrix3x3f*)cameraMatrix withOutR:(PSMMatrix3x3f*)outR andOutT:(PSMVector3f*)outT;
{
    NSAssert(objectPoints, @"objectPoints is null");
    NSAssert([objectPoints count] == 4, @"objectPoints array count is not 4");
    NSAssert(imagePoints, @"imagePoints is null");
    NSAssert([imagePoints count] == 4, @"imagePoints array count is not 4");
    NSAssert(cameraMatrix, @"cameraMatrix is null");
    NSAssert(outR, @"outR is null");
    NSAssert(outT, @"outT is null");

    psm::Vector3f objectPointsRaw[4];
    for (int i = 0; i < 4; ++i) {
        objectPointsRaw[i] = *static_cast<const psm::Vector3f*>([objectPoints[i] nativeVector3f]);
    }

    psm::Vector2f imagePointsRaw[4];
    for (int i = 0; i < 4; ++i) {
        imagePointsRaw[i] = *static_cast<const psm::Vector2f*>([imagePoints[i] nativeVector2f]);
    }

    const auto& cameraMatrixRaw = *static_cast<const psm::Matrix3x3f*>([cameraMatrix nativeMatrix3x3f]);
    auto& outRRaw = *static_cast<psm::Matrix3x3f*>([outR nativeMatrix3x3f]);
    auto& outTRaw = *static_cast<psm::Vector3f*>([outT nativeVector3f]);

    return psm::PoseEstimation::solvePnP(objectPointsRaw, imagePointsRaw, cameraMatrixRaw, outRRaw, outTRaw) ? YES : NO;
}

@end
