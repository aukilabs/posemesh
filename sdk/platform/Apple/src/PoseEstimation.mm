#import <Posemesh/PoseEstimation.h>

#include <Posemesh/PoseEstimation.hpp>

@implementation PSMPoseEstimation

+ (PSMPose*)solvePnPForLandmarks:(NSArray<PSMLandmark*>*)landmarks andLandmarkObservations:(NSArray<PSMLandmarkObservation*>*)landmarkObservations andCameraMatrix:(PSMMatrix3x3*)cameraMatrix withMethod:(PSMSolvePnpMethod)method;
{
    NSAssert(landmarks, @"landmarks is null");
    NSAssert(landmarkObservations, @"landmarkObservations is null");

    std::vector<psm::Landmark> landmarksRaw;
    for (int i = 0; i < landmarks.count; i++) {
        landmarksRaw.push_back(*static_cast<const psm::Landmark*>([landmarks[i] nativeLandmark]));
    }

    std::vector<psm::LandmarkObservation> landmarkObservationsRaw;
    for (int i = 0; i < landmarkObservations.count; i++) {
        landmarkObservationsRaw.push_back(*static_cast<const psm::LandmarkObservation*>([landmarkObservations[i] nativeLandmarkObservation]));
    }

    const auto& cameraMatrixRaw = *static_cast<const psm::Matrix3x3*>([cameraMatrix nativeMatrix3x3]);
    psm::Pose poseRaw = psm::PoseEstimation::solvePnP(landmarksRaw, landmarkObservationsRaw, cameraMatrixRaw, (psm::SolvePnpMethod)method);

    PSMPose* pose = [[PSMPose alloc] init];

    psm::Vector3 p = poseRaw.getPosition();
    PSMVector3* position = [[PSMVector3 alloc] init];
    [position setX:p.getX()];
    [position setY:p.getY()];
    [position setZ:p.getZ()];
    [pose setPosition:position];

    psm::Quaternion rotationRaw = poseRaw.getRotation();
    PSMQuaternion* rotation = [[PSMQuaternion alloc] init];
    [rotation setX:rotationRaw.getX()];
    [rotation setY:rotationRaw.getY()];
    [rotation setZ:rotationRaw.getZ()];
    [rotation setW:rotationRaw.getW()];
    [pose setRotation:rotation];

    return pose;
}

+ (PSMPose*)solvePnPCameraPoseForLandmarks:(NSArray<PSMLandmark*>*)landmarks andLandmarkObservations:(NSArray<PSMLandmarkObservation*>*)landmarkObservations andCameraMatrix:(PSMMatrix3x3*)cameraMatrix withMethod:(PSMSolvePnpMethod)method;
{
    NSAssert(landmarks, @"landmarks is null");
    NSAssert(landmarkObservations, @"landmarkObservations is null");

    std::vector<psm::Landmark> landmarksRaw;
    for (int i = 0; i < landmarks.count; i++) {
        landmarksRaw.push_back(*static_cast<const psm::Landmark*>([landmarks[i] nativeLandmark]));
    }

    std::vector<psm::LandmarkObservation> landmarkObservationsRaw;
    for (int i = 0; i < landmarkObservations.count; i++) {
        landmarkObservationsRaw.push_back(*static_cast<const psm::LandmarkObservation*>([landmarkObservations[i] nativeLandmarkObservation]));
    }

    const auto& cameraMatrixRaw = *static_cast<const psm::Matrix3x3*>([cameraMatrix nativeMatrix3x3]);
    psm::Pose poseRaw = psm::PoseEstimation::solvePnPCameraPose(landmarksRaw, landmarkObservationsRaw, cameraMatrixRaw, (psm::SolvePnpMethod)method);

    PSMPose* pose = [[PSMPose alloc] init];

    psm::Vector3 p = poseRaw.getPosition();
    PSMVector3* position = [[PSMVector3 alloc] init];
    [position setX:p.getX()];
    [position setY:p.getY()];
    [position setZ:p.getZ()];
    [pose setPosition:position];

    psm::Quaternion rotationRaw = poseRaw.getRotation();
    PSMQuaternion* rotation = [[PSMQuaternion alloc] init];
    [rotation setX:rotationRaw.getX()];
    [rotation setY:rotationRaw.getY()];
    [rotation setZ:rotationRaw.getZ()];
    [rotation setW:rotationRaw.getW()];
    [pose setRotation:rotation];

    return pose;
}

@end
