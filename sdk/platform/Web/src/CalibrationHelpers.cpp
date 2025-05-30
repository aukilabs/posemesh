#include <Posemesh/CalibrationHelpers.hpp>
#include <cassert>
#include <emscripten/bind.h>
#include <memory>

using namespace emscripten;
using namespace psm;

namespace {
std::shared_ptr<psm::Matrix4x4> getCalibrationMatrix(const Pose& domain, const Pose& observed, bool onlyRotateAroundY)
{
    return std::make_shared<psm::Matrix4x4>(CalibrationHelpers::getCalibrationMatrix(domain, observed, onlyRotateAroundY));
}
}

EMSCRIPTEN_BINDINGS(CalibrationHelpers)
{
    class_<CalibrationHelpers>("CalibrationHelpers")
        .class_function("__getCalibrationMatrix(domain, observed, onlyRotateAroundY)", &getCalibrationMatrix, nonnull<ret_val>());
}
