#ifndef __POSEMESH_POSE_ESTIMATION_HPP__
#define __POSEMESH_POSE_ESTIMATION_HPP__

#include <Posemesh/Matrix3x3.hpp>
#include <Posemesh/Vector2.hpp>
#include <Posemesh/Vector3.hpp>

#include "API.hpp"

namespace psm {

class PoseEstimation final {
public:
    static bool PSM_API solvePnP(
        const Vector3 objectPoints[],
        const Vector2 imagePoints[],
        const Matrix3x3& cameraMatrix,
        Matrix3x3& outR,
        Vector3& outT);

private:
    PoseEstimation() = delete;
};

}

#endif // __POSEMESH_POSE_ESTIMATION_HPP__
