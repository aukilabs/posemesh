#include <Posemesh/C/QRDetection.h>
#include <Posemesh/QRDetection.hpp>

bool PSM_API psm_qr_detection_detect_qr(
    uint8_t* image,
    int width,
    int height,
    char** contents,
    psm_vector2_t*** corners)
{
    const uint8_t* imagePointsRawBytes = static_cast<const uint8_t*>(image);
    std::vector<uint8_t> bytes(imagePointsRawBytes, imagePointsRawBytes + width * height);

    std::vector<std::string> outContents;
    std::vector<psm::Vector2> outCorners;
    bool result = psm::QRDetection::detectQRFromLuminance(bytes, width, height, outContents, outCorners);

    if (result) {
        std::string content = outContents[0];
        *contents = (char*)malloc(content.size() + 1);
        strcpy(*contents, content.c_str());

        *corners = (psm_vector2_t**)malloc(outCorners.size() * sizeof(psm_vector2_t*));
        for (int i = 0; i < outCorners.size(); i++) {
            psm_vector2_t* v = psm_vector2_create();
            psm_vector2_set_x(v, outCorners[i].getX());
            psm_vector2_set_y(v, outCorners[i].getY());
            (*corners)[i] = v;
        }
    }

    return result;
}
