#ifndef __POSEMESH_QR_DETECTION_HPP__
#define __POSEMESH_QR_DETECTION_HPP__

#include "API.hpp"
#include <Posemesh/Vector2f.hpp>
#include <Posemesh/Vector3f.hpp>
#include <vector>

namespace psm {

class QRDetection final {
public:
    static bool PSM_API detectQR(
        const std::vector<Vector3f>& image,
        int width,
        int height);
    static bool PSM_API detectQRFromLuminance(
        const std::vector<uint8_t>& imageBytes,
        int width,
        int height,
        std::vector<std::string>& contents,
        std::vector<Vector2f>& corners);

private:
    QRDetection() = delete;
};

}

#endif // __POSEMESH_QR_DETECTION_HPP__