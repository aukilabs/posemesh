#ifndef __POSEMESH_QR_DETECTION_HPP__
#define __POSEMESH_QR_DETECTION_HPP__

#include "API.hpp"
#include <vector>
#include <Posemesh/Vector3f.hpp>

namespace psm {

class QRDetection final {
public:
    static bool PSM_API detectQR(
        const std::vector<Vector3f>& image,
        int width,
        int height);

private:
    QRDetection() = delete;
};

}

#endif // __POSEMESH_QR_DETECTION_HPP__ 