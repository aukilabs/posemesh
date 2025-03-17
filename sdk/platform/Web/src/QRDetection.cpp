#include <Posemesh/QRDetection.hpp>
#include <emscripten/bind.h>
#include <memory>

using namespace emscripten;
using namespace psm;

namespace {
bool detectQRFromLuminance(
    const std::vector<uint8_t>& imageBytes,
    int width,
    int height,
    std::vector<std::string>& outContents,
    std::vector<std::shared_ptr<Vector2>>& outCorners)
{
    std::vector<Vector2> corners;
    bool detectionResult = QRDetection::detectQRFromLuminance(imageBytes, width, height, outContents, corners);

    outCorners.clear();
    for (auto& corner : corners) {
        outCorners.emplace_back(new Vector2(std::move(corner)));
    }

    return detectionResult;
}
}

EMSCRIPTEN_BINDINGS(QRDetection)
{
    class_<QRDetection>("QRDetection")
        .class_function("__detectQRFromLuminance(imageBytes, width, height, outContents, outCorners)", &detectQRFromLuminance);
}
