#include <Posemesh/ArucoDetection.hpp>
#include <iostream>
#include <opencv2/imgproc.hpp>
#include <opencv2/objdetect.hpp>
#include <opencv2/objdetect/aruco_detector.hpp>
#include <stdexcept>
#include <utility>

namespace psm {

cv::aruco::PredefinedDictionaryType toCvArucoPredefinedDictionaryType(psm::ArucoMarkerFormat format)
{
    switch (format) {
    case ArucoMarkerFormat::Set4x4Codes50:
        return cv::aruco::PredefinedDictionaryType::DICT_4X4_50;
    case ArucoMarkerFormat::Set4x4Codes100:
        return cv::aruco::PredefinedDictionaryType::DICT_4X4_100;
    case ArucoMarkerFormat::Set4x4Codes250:
        return cv::aruco::PredefinedDictionaryType::DICT_4X4_250;
    case ArucoMarkerFormat::Set4x4Codes1000:
        return cv::aruco::PredefinedDictionaryType::DICT_4X4_1000;
    case ArucoMarkerFormat::Set5x5Codes50:
        return cv::aruco::PredefinedDictionaryType::DICT_5X5_50;
    case ArucoMarkerFormat::Set5x5Codes100:
        return cv::aruco::PredefinedDictionaryType::DICT_5X5_100;
    case ArucoMarkerFormat::Set5x5Codes250:
        return cv::aruco::PredefinedDictionaryType::DICT_5X5_250;
    case ArucoMarkerFormat::Set5x5Codes1000:
        return cv::aruco::PredefinedDictionaryType::DICT_5X5_1000;
    case ArucoMarkerFormat::Set6x6Codes50:
        return cv::aruco::PredefinedDictionaryType::DICT_6X6_50;
    case ArucoMarkerFormat::Set6x6Codes100:
        return cv::aruco::PredefinedDictionaryType::DICT_6X6_100;
    case ArucoMarkerFormat::Set6x6Codes250:
        return cv::aruco::PredefinedDictionaryType::DICT_6X6_250;
    case ArucoMarkerFormat::Set6x6Codes1000:
        return cv::aruco::PredefinedDictionaryType::DICT_6X6_1000;
    case ArucoMarkerFormat::Set7x7Codes50:
        return cv::aruco::PredefinedDictionaryType::DICT_7X7_50;
    case ArucoMarkerFormat::Set7x7Codes100:
        return cv::aruco::PredefinedDictionaryType::DICT_7X7_100;
    case ArucoMarkerFormat::Set7x7Codes250:
        return cv::aruco::PredefinedDictionaryType::DICT_7X7_250;
    case ArucoMarkerFormat::Set7x7Codes1000:
        return cv::aruco::PredefinedDictionaryType::DICT_7X7_1000;
    case ArucoMarkerFormat::SetArucoOriginal:
        return cv::aruco::PredefinedDictionaryType::DICT_ARUCO_ORIGINAL;
    case ArucoMarkerFormat::SetApriltagCodes16h5:
        return cv::aruco::PredefinedDictionaryType::DICT_APRILTAG_16h5;
    case ArucoMarkerFormat::SetApriltagCodes25h9:
        return cv::aruco::PredefinedDictionaryType::DICT_APRILTAG_25h9;
    case ArucoMarkerFormat::SetApriltagCodes36h10:
        return cv::aruco::PredefinedDictionaryType::DICT_APRILTAG_36h10;
    case ArucoMarkerFormat::SetApriltagCodes36h11:
        return cv::aruco::PredefinedDictionaryType::DICT_APRILTAG_36h11;
    case ArucoMarkerFormat::SetArucoMipCodes36h12:
        return cv::aruco::PredefinedDictionaryType::DICT_ARUCO_MIP_36h12;

    default:
        throw std::invalid_argument("Invalid ArucoMarkerFormat");
    }
}

bool ArucoDetection::detectArucoFromLuminance(
    const std::uint8_t* imageBytes,
    std::size_t imageBytesSize,
    int width,
    int height,
    ArucoMarkerFormat markerFormat,
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
        cv::aruco::Dictionary dictionary = cv::aruco::getPredefinedDictionary(toCvArucoPredefinedDictionaryType(markerFormat));
        cv::aruco::ArucoDetector detector(dictionary, detectorParams);

        std::vector<std::vector<cv::Point2f>> cornersFound;
        std::vector<int> contentsFound;

        detector.detectMarkers(cvImage, cornersFound, contentsFound);

        if (contentsFound.empty()) {
            return false;
        }

        outContents.clear();
        outContents.reserve(contentsFound.size());

        outCorners.clear();
        outCorners.reserve(cornersFound.size());

        for (std::size_t i = 0; i < contentsFound.size(); ++i) {
            outContents.push_back(std::to_string(contentsFound[i]));

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
