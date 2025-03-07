#include <Posemesh/C/QRDetection.h>
#include <Posemesh/QRDetection.hpp>

bool psm_qr_detection_detect_qr(
    psm_vector3_t image[],
    int width,
    int height)
{
    std::vector<psm::Vector3> image_vector;
    image_vector.resize(width * height);
    for (int i = 0; i < width * height; i++) {
        image_vector[i] = image[i];
    }

    // TODO: Implement C binding.
    return false;
    //psm::Vector2f corners[4];
    //bool result = psm::QRDetection::detectQR(image, width, height, corners);
    // bool result = static_cast<bool>(psm::QRDetection::detectQR(image_vector, width, height));
    
    /*
    if (result) {
        for (int i = 0; i < 4; i++) {
            outCorners[i].setX(corners[i].getX());
            outCorners[i].setY(corners[i].getY());
        }
    }
    */

    // return result;
} 