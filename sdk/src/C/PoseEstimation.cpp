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
        iPoints[i] = *(imagePoints[i]);
        oPoints[i] = *(objectPoints[i]);
    }

    return static_cast<uint8_t>(psm::PoseEstimation::solvePnP(
        oPoints,
        iPoints,
        *cameraMatrix,
        *outR,
        *outT));
}
