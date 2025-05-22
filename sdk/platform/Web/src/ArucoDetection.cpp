#include <Posemesh/ArucoDetection.hpp>
#include <emscripten/bind.h>
#include <memory>

using namespace emscripten;
using namespace psm;

namespace {
bool detectArucoFromLuminance(
    const std::vector<uint8_t>& imageBytes,
    int width,
    int height,
    int markerFormat,
    std::vector<std::string>& outContents,
    std::vector<std::shared_ptr<Vector2>>& outCorners)
{
    std::vector<Vector2> corners;
    bool detectionResult = ArucoDetection::detectArucoFromLuminance(imageBytes.data(), imageBytes.size(), width, height, (ArucoMarkerFormat)markerFormat, outContents, corners);

    outCorners.clear();
    for (auto& corner : corners) {
        outCorners.emplace_back(new Vector2(std::move(corner)));
    }

    return detectionResult;
}

std::vector<std::shared_ptr<psm::LandmarkObservation>> detectArucoFromLuminanceLandmarkObservations(
    const std::vector<uint8_t>& imageBytes,
    int width,
    int height,
    int markerFormat)
{
    std::vector<std::shared_ptr<psm::LandmarkObservation>> observations;
    auto lo = ArucoDetection::detectArucoFromLuminance(imageBytes, width, height, (ArucoMarkerFormat)markerFormat);
    for (auto& observation : lo) {
        observations.emplace_back(new psm::LandmarkObservation(std::move(observation)));
    }

    return observations;
}
}

EMSCRIPTEN_BINDINGS(ArucoDetection)
{
    class_<ArucoDetection>("ArucoDetection")
        .class_function("__detectArucoFromLuminance(imageBytes, width, height, markerFormat, outContents, outCorners)", &detectArucoFromLuminance)
        .class_function("__detectArucoFromLuminanceLandmarkObservations(imageBytes, width, height, markerFormat)", &detectArucoFromLuminanceLandmarkObservations);
}
