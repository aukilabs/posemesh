#include <Posemesh/QRDetection.hpp>
#include <emscripten/bind.h>
#include <memory>

using namespace emscripten;
using namespace psm;

namespace {
bool detectQR(
    const std::vector<Vector3f>& image,
    int width,
    int height)
{
    return QRDetection::detectQR(image, width, height);
}
}

EMSCRIPTEN_BINDINGS(QRDetection)
{
    class_<QRDetection>("QRDetection")
        .class_function("detectQR(image, width, height)", &detectQR);
} 