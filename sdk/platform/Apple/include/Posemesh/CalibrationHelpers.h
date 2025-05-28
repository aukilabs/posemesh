#import <Foundation/Foundation.h>

#import "API.h"
#import "Matrix4x4.h"
#import "Pose.h"

NS_SWIFT_NAME(CalibrationHelpers) PSM_API @interface PSMCalibrationHelpers : NSObject

- (instancetype _Nonnull)init NS_UNAVAILABLE;
- (instancetype _Nonnull)copy NS_UNAVAILABLE;

+ (PSMMatrix4x4* _Nonnull)calibrationMatrixWithPoseInWorld:(PSMPose*)inWorld andInDomain:(PSMPose*)inDomain onlyRotateAroundY:(BOOL)onlyRotateAroundY NS_REFINED_FOR_SWIFT;

@end
