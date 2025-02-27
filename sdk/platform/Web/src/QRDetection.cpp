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

bool detectQRFromLuminance(
    const std::vector<uint8_t>& imageBytes,
    int width,
    int height,
    std::vector<std::string>& contents,
    std::vector<Vector2f>& corners)
{
    return QRDetection::detectQRFromLuminance(imageBytes, width, height, contents, corners);
}
}

EMSCRIPTEN_BINDINGS(QRDetection)
{
    class_<QRDetection>("QRDetection")
        .class_function("detectQR(image, width, height)", &detectQR)
        .class_function("detectQRFromLuminance(bytes, width, height, contents, corners)", &detectQRFromLuminance);
}