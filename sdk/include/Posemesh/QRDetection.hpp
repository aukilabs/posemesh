#ifndef __POSEMESH_QR_DETECTION_HPP__
#define __POSEMESH_QR_DETECTION_HPP__

#include "API.hpp"
#include <Posemesh/Vector2.hpp>
#include <Posemesh/Vector3.hpp>
#include <cstdint>
#include <string>
#include <vector>

namespace psm {

class QRDetection final {
public:
    static bool PSM_API detectQRFromLuminance(
        const std::vector<std::uint8_t>& imageBytes,
        int width,
        int height,
        std::vector<std::string>& contents,
        std::vector<Vector2>& corners);

private:
    QRDetection() = delete;
};

}

#endif // __POSEMESH_QR_DETECTION_HPP__