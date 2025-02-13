#include <Posemesh/PoseEstimation.hpp>
#include <emscripten/bind.h>
#include <memory>

using namespace emscripten;
using namespace psm;

EMSCRIPTEN_BINDINGS(PoseEstimation)
{
    class_<PoseEstimation>("PoseEstimation")
        .class_function("__getSolvePnP()", &PoseEstimation::getSolvePnP);
}
