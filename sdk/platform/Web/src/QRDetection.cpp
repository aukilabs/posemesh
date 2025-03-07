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
    std::vector<std::string>& contents,
    std::vector<std::shared_ptr<Vector2>>& corners)
{
    std::vector<Vector2> outputCorners;
    std::vector<std::string> outputContents;
    bool detectionResult = QRDetection::detectQRFromLuminance(imageBytes, width, height, outputContents, outputCorners);
    
    for (size_t i = 0; i < outputContents.size(); i++) {
        contents.push_back(outputContents[i]);
    }

    for (size_t i = 0; i < outputCorners.size(); i++) {
        std::shared_ptr<Vector2> c(new Vector2());
        c->setX(outputCorners[i].getX());
        c->setY(outputCorners[i].getY());
        corners.push_back(c);
    }

    return detectionResult;
}
}

EMSCRIPTEN_BINDINGS(QRDetection)
{
    class_<QRDetection>("QRDetection")
        .class_function("__detectQRFromLuminance(bytes, width, height, contents, corners)", &detectQRFromLuminance);
}