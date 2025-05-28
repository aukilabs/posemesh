#include <Posemesh/PoseEstimation.hpp>
#include <emscripten/bind.h>
#include <iostream>
#include <memory>
#include <vector>

using namespace emscripten;
using namespace psm;

namespace {
Pose solvePnP(const std::vector<std::shared_ptr<Landmark>>& landmarks,
    const std::vector<std::shared_ptr<LandmarkObservation>>& landmarkObservations,
    const Matrix3x3& cameraMatrix,
    int method)
{
    std::vector<Landmark> landmarksRaw;
    for (int i = 0; i < landmarks.size(); ++i) {
        landmarksRaw.push_back(*landmarks[i]);
    }

    std::vector<LandmarkObservation> landmarkObservationsRaw;
    for (int i = 0; i < landmarkObservations.size(); ++i) {
        landmarkObservationsRaw.push_back(*landmarkObservations[i]);
    }

    return PoseEstimation::solvePnP(landmarksRaw, landmarkObservationsRaw, cameraMatrix, (psm::SolvePnpMethod)method);
}
}

EMSCRIPTEN_BINDINGS(PoseEstimation)
{
    class_<PoseEstimation>("PoseEstimation")
        .class_function("__solvePnP(landmarks, landmarkObservations, cameraMatrix, method)", &solvePnP);
}
