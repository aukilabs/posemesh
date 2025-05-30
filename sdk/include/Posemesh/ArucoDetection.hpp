#ifndef __POSEMESH_ARUCO_DETECTION_HPP__
#define __POSEMESH_ARUCO_DETECTION_HPP__

#include "API.hpp"
#include "ArucoMarkerFormat.hpp"
#include "LandmarkObservation.hpp"
#include "Vector2.hpp"
#include "Vector3.hpp"
#include <cstdint>
#include <string>
#include <vector>

namespace psm {

class ArucoDetection final {
public:
    static bool PSM_API detectArucoFromLuminance(
        const std::uint8_t* imageBytes,
        std::size_t imageBytesSize,
        int width,
        int height,
        ArucoMarkerFormat markerFormat,
        std::vector<std::string>& outContents,
        std::vector<Vector2>& outCorners);

    static bool PSM_API detectArucoFromLuminance(
        const std::vector<std::uint8_t>& imageBytes,
        int width,
        int height,
        ArucoMarkerFormat markerFormat,
        std::vector<std::string>& outContents,
        std::vector<Vector2>& outCorners);

    static std::vector<LandmarkObservation> PSM_API detectArucoFromLuminance(
        const std::vector<std::uint8_t>& imageBytes,
        int width,
        int height,
        ArucoMarkerFormat markerFormat);

private:
    ArucoDetection() = delete;
};

}

#endif // __POSEMESH_ARUCO_DETECTION_HPP__
