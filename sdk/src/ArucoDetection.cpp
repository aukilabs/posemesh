#include <Posemesh/ArucoDetection.hpp>
#include <iostream>
#include <opencv2/imgproc.hpp>
#include <opencv2/objdetect.hpp>
#include <opencv2/objdetect/aruco_detector.hpp>
#include <stdexcept>
#include <utility>

namespace psm {

bool ArucoDetection::detectArucoFromLuminance(
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
        const cv::Mat cvImage(cv::Size(width, height), CV_8U, const_cast<std::uint8_t*>(imageBytes));

        cv::aruco::DetectorParameters detectorParams;
        cv::aruco::Dictionary dictionary = cv::aruco::getPredefinedDictionary(cv::aruco::DICT_ARUCO_ORIGINAL);
        cv::aruco::ArucoDetector detector(dictionary, detectorParams);

        std::vector<std::vector<cv::Point2f>> cornersFound;
        std::vector<int> contentsFound;

        detector.detectMarkers(cvImage, cornersFound, contentsFound);

        if (contentsFound.empty()) {
            return false; // No markers found
        }

        outContents.clear();
        outContents.reserve(contentsFound.size());

        outCorners.clear();
        outCorners.reserve(cornersFound.size());

        for (std::size_t i = 0; i < contentsFound.size(); ++i) {
            outContents.push_back(std::to_string(contentsFound[i]));

            // Each marker has 4 corners
            for (const auto& p : cornersFound[i]) {
                Vector2 corner;
                corner.setX(p.x);
                corner.setY(p.y);
                outCorners.push_back(std::move(corner));
            }
        }
    } catch (const cv::Exception& e) {
        std::cerr << "ArucoDetection::detectArucoFromLuminance(): An OpenCV exception occurred: " << e.what() << std::endl;
        return false;
    } catch (const std::exception& e) {
        std::cerr << "ArucoDetection::detectArucoFromLuminance(): An exception occurred: " << e.what() << std::endl;
        return false;
    }
    return true;
}

}
