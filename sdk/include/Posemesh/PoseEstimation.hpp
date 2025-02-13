#ifndef __POSEMESH_POSE_ESTIMATION_HPP__
#define __POSEMESH_POSE_ESTIMATION_HPP__

#include "API.hpp"
#include <Posemesh/Vector3f.hpp>
#include <Posemesh/Matrix3x3f.hpp>

namespace psm {

class PoseEstimation final {
public:
    PSM_API ~PoseEstimation();

    static bool PSM_API solvePnP(
        const float* objectPoints,
        const float* imagePoints,
        const float* cameraMatrix,
        Matrix3x3f* outR,
        Vector3f* outT);
private:
    PoseEstimation() = delete;
};

}

#endif // __POSEMESH_POSE_ESTIMATION_HPP__
