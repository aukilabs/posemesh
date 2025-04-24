#ifndef __POSEMESH_POSE_ESTIMATION_HPP__
#define __POSEMESH_POSE_ESTIMATION_HPP__

#include <Posemesh/Landmark.hpp>
#include <Posemesh/LandmarkObservation.hpp>
#include <Posemesh/Matrix3x3.hpp>
#include <Posemesh/Pose.hpp>
#include <Posemesh/SolvePnPMethod.hpp>
#include <Posemesh/Vector2.hpp>
#include <Posemesh/Vector3.hpp>

#include "API.hpp"

namespace psm {

class PoseEstimation final {
public:
    static Pose PSM_API solvePnP(
        const std::vector<Landmark>& landmarks,
        const std::vector<LandmarkObservation>& landmarkObservations,
        const Matrix3x3& cameraMatrix,
        SolvePnpMethod method);

private:
    PoseEstimation() = delete;
};

}

#endif // __POSEMESH_POSE_ESTIMATION_HPP__
