#import <Posemesh/PoseEstimation.h>

#include <Posemesh/PoseEstimation.hpp>

@implementation PSMPoseEstimation

+ (BOOL)solvePnPForObjectPoints:(NSArray *)objectPoints imagePoints:(NSArray *)imagePoints cameraMatrix:(PSMMatrix3x3f *)cameraMatrix outR:(PSMMatrix3x3f *)outR outT:(PSMVector3f *)outT {
    psm::Vector3f oPoints[4];
    for (int i = 0; i < 4; i++) {
        oPoints[i].setX([objectPoints[i] x]);
        oPoints[i].setY([objectPoints[i] y]);
        oPoints[i].setZ([objectPoints[i] z]);
    }

    psm::Vector2f iPoints[4];
    for (int i = 0; i < 4; i++) {
        iPoints[i].setX([imagePoints[i] x]);
        iPoints[i].setY([imagePoints[i] y]);
    }

    psm::Matrix3x3f cMatrix;
    cMatrix.setM00([cameraMatrix m00]);
    cMatrix.setM01([cameraMatrix m01]);
    cMatrix.setM02([cameraMatrix m02]);
    cMatrix.setM10([cameraMatrix m10]);
    cMatrix.setM11([cameraMatrix m11]);
    cMatrix.setM12([cameraMatrix m12]);
    cMatrix.setM20([cameraMatrix m20]);
    cMatrix.setM21([cameraMatrix m21]);
    cMatrix.setM22([cameraMatrix m22]);

    psm::Vector3f translation;
    psm::Matrix3x3f rotation;
    bool estimationResult = psm::PoseEstimation::solvePnP(oPoints, iPoints, cMatrix, &rotation, &translation);
    if (estimationResult == false) {
        return false;
    }

    [outT setX:translation.getX()];
    [outT setY:translation.getY()];
    [outT setZ:translation.getZ()];

    [outR setM00:rotation.getM00()];
    [outR setM01:rotation.getM01()];
    [outR setM02:rotation.getM02()];
    [outR setM10:rotation.getM10()];
    [outR setM11:rotation.getM11()];
    [outR setM12:rotation.getM12()];
    [outR setM20:rotation.getM20()];
    [outR setM21:rotation.getM21()];
    [outR setM22:rotation.getM22()];
    
    return estimationResult;
}


@end
