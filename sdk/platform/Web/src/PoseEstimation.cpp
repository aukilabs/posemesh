#include <Posemesh/PoseEstimation.hpp>
#include <emscripten/bind.h>
#include <memory>

using namespace emscripten;
using namespace psm;

namespace {
bool solvePnP(
    std::vector<Vector3f> objectPoints,
    std::vector<Vector2f> imagePoints,
    std::shared_ptr<Matrix3x3f> cameraMatrix,
    std::shared_ptr<Matrix3x3f> outR,
    std::shared_ptr<Vector3f> outT)
{
    return PoseEstimation::solvePnP(objectPoints.data(), imagePoints.data(), *cameraMatrix, outR.get(), outT.get());
}
}

EMSCRIPTEN_BINDINGS(PoseEstimation)
{
    class_<PoseEstimation>("PoseEstimation")
        .class_function("solvePnP(objectPoints, imagePoints, cameraMatrix, outR, outT)", &solvePnP);
}
