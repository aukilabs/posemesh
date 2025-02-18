#include <Posemesh/C/PoseEstimation.h>
#include <Posemesh/PoseEstimation.hpp>

bool psm_pose_estimation_get_solve_pnp(
    psm_vector3f_t* objectPoints[],
    psm_vector2f_t* imagePoints[],
    psm_matrix3x3f_t* cameraMatrix,
    psm_matrix3x3f_t* outR,
    psm_vector3f_t* outT)
{
    psm::Vector3f oPoints[4];
    for (int i = 0; i < 4; i++) {
        oPoints[i].setX(objectPoints[i]->getX());
        oPoints[i].setY(objectPoints[i]->getY());
        oPoints[i].setZ(objectPoints[i]->getZ());
    }

    psm::Vector2f iPoints[4];
    for (int i = 0; i < 4; i++) {
        iPoints[i].setX(imagePoints[i]->getX());
        iPoints[i].setY(imagePoints[i]->getY());
    }

    psm::Matrix3x3f cMatrix;
    cMatrix.setM00(cameraMatrix->getM00());
    cMatrix.setM01(cameraMatrix->getM01());
    cMatrix.setM02(cameraMatrix->getM02());
    cMatrix.setM10(cameraMatrix->getM10());
    cMatrix.setM11(cameraMatrix->getM11());
    cMatrix.setM12(cameraMatrix->getM12());
    cMatrix.setM20(cameraMatrix->getM20());
    cMatrix.setM21(cameraMatrix->getM21());
    cMatrix.setM22(cameraMatrix->getM22());

    psm::Vector3f translation;
    psm::Matrix3x3f rotation;

    bool estimationResult = static_cast<bool>(psm::PoseEstimation::solvePnP(
        oPoints,
        iPoints,
        cMatrix,
        &rotation,
        &translation));

    if (estimationResult == false) {
        return false;
    }

    // from oT & oR -> outT & outR
    
    outT->setX(translation.getX());
    outT->setY(translation.getY());
    outT->setZ(translation.getZ());

    outR->setM00(rotation.getM00());
    outR->setM01(rotation.getM01());
    outR->setM02(rotation.getM02());
    outR->setM10(rotation.getM10());
    outR->setM11(rotation.getM11());
    outR->setM12(rotation.getM12());
    outR->setM20(rotation.getM20());
    outR->setM21(rotation.getM21());
    outR->setM22(rotation.getM22());

    return estimationResult;
}
