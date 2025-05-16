#include <Posemesh/CalibrationHelpers.hpp>
#include <cassert>
#include <emscripten/bind.h>
#include <memory>

using namespace emscripten;
using namespace psm;

namespace {
std::shared_ptr<psm::Matrix4x4> getCalibrationMatrix(const Pose& poseInDomain, const Pose& observedPose, bool onlyRotateAroundY)
{
    return std::make_shared<psm::Matrix4x4>(CalibrationHelpers::getCalibrationMatrix(poseInDomain, observedPose, onlyRotateAroundY));
}
}

EMSCRIPTEN_BINDINGS(CalibrationHelpers)
{
    class_<CalibrationHelpers>("CalibrationHelpers")
        .class_function("__getCalibrationMatrix(poseInDomain, observedPose, onlyRotateAroundY)", &getCalibrationMatrix, nonnull<ret_val>());
}
