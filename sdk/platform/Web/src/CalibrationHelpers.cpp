#include <Posemesh/CalibrationHelpers.hpp>
#include <cassert>
#include <emscripten/bind.h>
#include <memory>

using namespace emscripten;
using namespace psm;

namespace {
std::shared_ptr<psm::Matrix4x4> getCalibrationMatrix(const Pose& inWorld, const Pose& inDomain, bool onlyRotateAroundY)
{
    return std::make_shared<psm::Matrix4x4>(CalibrationHelpers::getCalibrationMatrix(inWorld, inDomain, onlyRotateAroundY));
}
}

EMSCRIPTEN_BINDINGS(CalibrationHelpers)
{
    class_<CalibrationHelpers>("CalibrationHelpers")
        .class_function("__getCalibrationMatrix(poseInWorld, poseInDomain, onlyRotateAroundY)", &getCalibrationMatrix, nonnull<ret_val>());
}
