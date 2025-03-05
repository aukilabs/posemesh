#include <Posemesh/C/PoseEstimation.h>
#include <Posemesh/PoseEstimation.hpp>

uint8_t psm_pose_estimation_solve_pnp(
    const psm_vector3f_t* objectPoints[],
    const psm_vector2f_t* imagePoints[],
    const psm_matrix3x3f_t* cameraMatrix,
    psm_matrix3x3f_t* outR,
    psm_vector3f_t* outT)
{
    psm::Vector3f oPoints[4];
    psm::Vector2f iPoints[4];

    for (int i = 0; i < 4; i++) {
        psm::Vector2f ip;
        ip.setX(imagePoints[i]->getX());
        ip.setY(imagePoints[i]->getY());
        iPoints[i] = ip;

        psm::Vector3f op;
        op.setX(objectPoints[i]->getX());
        op.setY(objectPoints[i]->getY());
        op.setZ(objectPoints[i]->getZ());
        oPoints[i] = op;
    }

    return static_cast<uint8_t>(psm::PoseEstimation::solvePnP(
        oPoints,
        iPoints,
        *cameraMatrix,
        *outR,
        *outT));
}
