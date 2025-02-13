#include <Posemesh/C/PoseEstimation.h>
#include <Posemesh/PoseEstimation.hpp>

bool psm_pose_estimation_get_solve_pn_p(
    const float* objectPoints,
    const float* imagePoints,
    const float* cameraMatrix,
    Matrix3x3f* outR,
    Vector3f* outT)
{
    return static_cast<bool>(psm::PoseEstimation::solvePnP(
        objectPoints,
        imagePoints,
        cameraMatrix,
        outR,
        outT));
}
