#include <Posemesh/PoseEstimation.hpp>
#include <emscripten/bind.h>
#include <iostream>
#include <memory>
#include <vector>

using namespace emscripten;
using namespace psm;

namespace {
bool solvePnP(const std::vector<std::shared_ptr<Vector3>>& objectPoints,
    const std::vector<std::shared_ptr<Vector2>>& imagePoints,
    const Matrix3x3& cameraMatrix,
    Matrix3x3& outR,
    Vector3& outT,
    int method)
{
    if (objectPoints.size() != 4) {
        std::cerr << "Posemesh.solvePnP(): objectPoints array length is not 4" << std::endl;
        return false;
    }
    if (imagePoints.size() != 4) {
        std::cerr << "Posemesh.solvePnP(): imagePoints array length is not 4" << std::endl;
        return false;
    }

    Vector3 objectPointsRaw[4];
    for (int i = 0; i < 4; ++i) {
        objectPointsRaw[i] = *(objectPoints[i]);
    }

    Vector2 imagePointsRaw[4];
    for (int i = 0; i < 4; ++i) {
        imagePointsRaw[i] = *(imagePoints[i]);
    }

    return PoseEstimation::solvePnP(objectPointsRaw, imagePointsRaw, cameraMatrix, outR, outT, (psm::SolvePnpMethod)method);
}
}

EMSCRIPTEN_BINDINGS(PoseEstimation)
{
    class_<PoseEstimation>("PoseEstimation")
        .class_function("__solvePnP(objectPoints, imagePoints, cameraMatrix, outR, outT, method)", &solvePnP);
}
