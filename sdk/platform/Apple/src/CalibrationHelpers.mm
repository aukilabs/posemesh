#import <Posemesh/CalibrationHelpers.h>

#include <Posemesh/CalibrationHelpers.hpp>
#include <new>

@implementation PSMCalibrationHelpers

+ (PSMMatrix4x4*)calibrationMatrixWithPoseInWorld:(PSMPose*)inWorld andInDomain:(PSMPose*)inDomain onlyRotateAroundY:(BOOL)onlyRotateAroundY
{
    return [[PSMMatrix4x4 alloc] initWithNativeMatrix4x4:new (std::nothrow) psm::Matrix4x4(psm::CalibrationHelpers::getCalibrationMatrix(*static_cast<const psm::Pose*>([inWorld nativePose]), *static_cast<const psm::Pose*>([inDomain nativePose]), onlyRotateAroundY))];
}

@end
