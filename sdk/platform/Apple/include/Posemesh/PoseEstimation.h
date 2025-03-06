#import <Foundation/Foundation.h>

#import "API.h"
#import "Matrix3x3.h"
#import "Vector2.h"
#import "Vector3.h"

NS_SWIFT_NAME(PoseEstimation) PSM_API @interface PSMPoseEstimation : NSObject

- (instancetype)init NS_UNAVAILABLE;
- (instancetype)copy NS_UNAVAILABLE;

+ (BOOL)solvePnPForObjectPoints:(NSArray<PSMVector3*>*)objectPoints andImagePoints:(NSArray<PSMVector2*>*)imagePoints andCameraMatrix:(PSMMatrix3x3*)cameraMatrix withOutR:(PSMMatrix3x3*)outR andOutT:(PSMVector3*)outT;

@end
