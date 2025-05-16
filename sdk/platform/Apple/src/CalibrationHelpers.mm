#import <Posemesh/CalibrationHelpers.h>

#include <Posemesh/CalibrationHelpers.hpp>
#include <new>

@implementation PSMCalibrationHelpers

+ (PSMMatrix4x4*)calibrationMatrixWithPoseInDomain:(PSMPose*)poseInDomain andObservedPose:(PSMPose*)observedPose onlyRotateAroundY:(BOOL)onlyRotateAroundY
{
    return [[PSMMatrix4x4 alloc] initWithNativeMatrix4x4:new (std::nothrow) psm::Matrix4x4(psm::CalibrationHelpers::getCalibrationMatrix(*static_cast<const psm::Pose*>([poseInDomain nativePose]), *static_cast<const psm::Pose*>([observedPose nativePose] ), onlyRotateAroundY))];
}

@end
