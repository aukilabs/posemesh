#import <Foundation/Foundation.h>

#import "API.h"
#import "Vector2f.h"
#import "Vector3f.h"
#import "Matrix3x3f.h"

NS_SWIFT_NAME(PoseEstimation) PSM_API @interface PSMPoseEstimation : NSObject

+ (BOOL)solvePnPForObjectPoints:(NSArray *)objectPoints imagePoints:(NSArray *)imagePoints cameraMatrix:(PSMMatrix3x3f *)cameraMatrix outR:(PSMMatrix3x3f *)outR outT:(PSMVector3f *)outT NS_REFINED_FOR_SWIFT;

@end
