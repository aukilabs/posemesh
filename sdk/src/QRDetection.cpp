#include <Posemesh/QRDetection.hpp>
#include <iostream>
#include <opencv2/imgproc.hpp>
#include <opencv2/objdetect.hpp>

namespace psm {

bool QRDetection::detectQRFromLuminance(
    const std::vector<uint8_t>& imageBytes,
    int width,
    int height,
    std::vector<std::string>& contents,
    std::vector<Vector2>& corners)
{
    cv::Mat cvImage(height, width, CV_8U);
    std::memcpy(cvImage.data, imageBytes.data(), width * height * sizeof(uint8_t));

    try {
        cv::QRCodeDetector qrDetector;
        std::vector<cv::Point2f> cornersFound;
        std::vector<std::string> contentsFound;
        bool detected = qrDetector.detectAndDecodeMulti(cvImage, contentsFound, cornersFound);
        if (detected) {
            for (size_t i = 0; i < contentsFound.size(); ++i) {
                contents.push_back(contentsFound[i]);
            }

            for (size_t i = 0; i < cornersFound.size(); ++i) {
                cv::Point2f p = cornersFound[i];
                Vector2 corner;
                corner.setX(p.x);
                corner.setY(p.y);
                corners.push_back(corner);
            }
        } else {
            return false;
        }
    } catch (cv::Exception& e) {
        std::cerr << "OpenCV exception caught: " << e.what() << std::endl;
        return false;
    }

    return true;
}
}