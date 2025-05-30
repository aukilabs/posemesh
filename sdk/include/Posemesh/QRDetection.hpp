#ifndef __POSEMESH_QR_DETECTION_HPP__
#define __POSEMESH_QR_DETECTION_HPP__

#include "API.hpp"
#include <Posemesh/LandmarkObservation.hpp>
#include <Posemesh/Vector2.hpp>
#include <Posemesh/Vector3.hpp>
#include <cstdint>
#include <string>
#include <vector>

namespace psm {

class QRDetection final {
public:
    static bool PSM_API detectQRFromLuminance(
        const std::uint8_t* imageBytes,
        std::size_t imageBytesSize,
        int width,
        int height,
        std::vector<std::string>& outContents,
        std::vector<Vector2>& outCorners);

    static bool PSM_API detectQRFromLuminance(
        const std::vector<std::uint8_t>& imageBytes,
        int width,
        int height,
        std::vector<std::string>& outContents,
        std::vector<Vector2>& outCorners);

    static std::vector<LandmarkObservation> PSM_API detectQRFromLuminance(
        const std::vector<std::uint8_t>& imageBytes,
        int width,
        int height);

private:
    QRDetection() = delete;
};

}

#endif // __POSEMESH_QR_DETECTION_HPP__
