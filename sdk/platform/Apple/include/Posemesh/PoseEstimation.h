#import <Foundation/Foundation.h>

#import "API.h"
#import "Matrix3x3f.h"
#import "Vector2f.h"
#import "Vector3f.h"

NS_SWIFT_NAME(PoseEstimation) PSM_API @interface PSMPoseEstimation : NSObject

- (instancetype)init NS_UNAVAILABLE;
- (instancetype)copy NS_UNAVAILABLE;

+ (BOOL)solvePnPForObjectPoints:(NSArray<PSMVector3f*>*)objectPoints andImagePoints:(NSArray<PSMVector2f*>*)imagePoints andCameraMatrix:(PSMMatrix3x3f*)cameraMatrix withOutR:(PSMMatrix3x3f*)outR andOutT:(PSMVector3f*)outT;

@end
