#import <Foundation/Foundation.h>

#import "API.h"
#import "Landmark.h"
#import "LandmarkObservation.h"
#import "Matrix3x3.h"
#import "Pose.h"
#import "SolvePnPMethod.h"
#import "Vector2.h"
#import "Vector3.h"

NS_SWIFT_NAME(PoseEstimation) PSM_API @interface PSMPoseEstimation : NSObject

- (instancetype)init NS_UNAVAILABLE;
- (instancetype)copy NS_UNAVAILABLE;

+ (PSMPose*)solvePnPForLandmarks:(NSArray<PSMLandmark*>*)landmarks andLandmarkObservations:(NSArray<PSMLandmarkObservation*>*)landmarkObservations andCameraMatrix:(PSMMatrix3x3*)cameraMatrix withMethod:(PSMSolvePnpMethod)method NS_REFINED_FOR_SWIFT;
+ (PSMPose*)solvePnPCameraPoseForLandmarks:(NSArray<PSMLandmark*>*)landmarks andLandmarkObservations:(NSArray<PSMLandmarkObservation*>*)landmarkObservations andCameraMatrix:(PSMMatrix3x3*)cameraMatrix withMethod:(PSMSolvePnpMethod)method NS_REFINED_FOR_SWIFT;
+ (PSMPose*)cameraPoseFromSolvePnPPose:(PSMPose*)solvePnPPose NS_REFINED_FOR_SWIFT;

@end
