#include <Posemesh/QRDetection.hpp>
#include <iostream>
#include <opencv2/imgproc.hpp>
#include <opencv2/objdetect.hpp>
#include <stdexcept>
#include <utility>

namespace psm {

bool QRDetection::detectQRFromLuminance(
    const std::uint8_t* imageBytes,
    std::size_t imageBytesSize,
    int width,
    int height,
    std::vector<std::string>& outContents,
    std::vector<Vector2>& outCorners)
{
    try {
        if (width <= 0) {
            throw std::invalid_argument("width");
        }
        if (height <= 0) {
            throw std::invalid_argument("height");
        }
        if (imageBytesSize != width * height * sizeof(std::uint8_t)) {
            throw std::invalid_argument("imageBytesSize");
        }

        // Hacky, but we aren't gonna modify cvImage so this way we skip unnecessary copying
        const cv::Mat cvImage(cv::Size(height, width), CV_8U, const_cast<std::uint8_t*>(imageBytes));

        const cv::QRCodeDetector qrDetector;
        std::vector<cv::Point2f> cornersFound;
        std::vector<std::string> contentsFound;
        const bool detected = qrDetector.detectAndDecodeMulti(cvImage, contentsFound, cornersFound);
        if (detected) {
            outContents.clear();
            outContents.reserve(contentsFound.size());
            for (const auto& s : contentsFound) {
                outContents.push_back(s);
            }

            outCorners.clear();
            outCorners.reserve(cornersFound.size());
            for (const auto& p : cornersFound) {
                Vector2 corner;
                corner.setX(p.x);
                corner.setY(p.y);
                outCorners.push_back(std::move(corner));
            }
        } else {
            return false;
        }
    } catch (const cv::Exception& e) {
        std::cerr << "QRDetection::detectQRFromLuminance(): An OpenCV exception occurred: " << e.what() << std::endl;
        return false;
    } catch (const std::exception& e) {
        std::cerr << "QRDetection::detectQRFromLuminance(): An exception occurred: " << e.what() << std::endl;
        return false;
    }

    return true;
}

bool QRDetection::detectQRFromLuminance(
    const std::vector<std::uint8_t>& imageBytes,
    int width,
    int height,
    std::vector<std::string>& outContents,
    std::vector<Vector2>& outCorners)
{
    return detectQRFromLuminance(imageBytes.data(), imageBytes.size(), width, height, outContents, outCorners);
}

}
