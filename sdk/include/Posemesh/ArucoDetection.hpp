#ifndef __POSEMESH_ARUCO_DETECTION_HPP__
#define __POSEMESH_ARUCO_DETECTION_HPP__

#include <Posemesh/Vector2.hpp>
#include <Posemesh/Vector3.hpp>
#include <cstdint>
#include <string>
#include <vector>

#include "API.hpp"
#include "ArucoMarkerFormat.hpp"

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

private:
    ArucoDetection() = delete;
};

}

#endif // __POSEMESH_ARUCO_DETECTION_HPP__
