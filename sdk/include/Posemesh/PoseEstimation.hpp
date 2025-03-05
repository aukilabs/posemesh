#ifndef __POSEMESH_POSE_ESTIMATION_HPP__
#define __POSEMESH_POSE_ESTIMATION_HPP__

#include <Posemesh/Matrix3x3f.hpp>
#include <Posemesh/Vector2f.hpp>
#include <Posemesh/Vector3f.hpp>

#include "API.hpp"

namespace psm {

class PoseEstimation final {
public:
    static bool PSM_API solvePnP(
        const Vector3f objectPoints[],
        const Vector2f imagePoints[],
        const Matrix3x3f& cameraMatrix,
        Matrix3x3f& outR,
        Vector3f& outT);

private:
    PoseEstimation() = delete;
};

}

#endif // __POSEMESH_POSE_ESTIMATION_HPP__
