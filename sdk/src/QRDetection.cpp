#include <Posemesh/QRDetection.hpp>
#include <iostream>
#include <opencv2/imgproc.hpp>
#include <opencv2/objdetect.hpp>

namespace psm {

bool QRDetection::detectQR(
    const std::vector<Vector3f>& image,
    int width,
    int height)
{
    cv::Mat cvImage(height, width, CV_8U);
    for (int i = 0; i < width * height; i++) {
        const auto& vec = image[i];
        float gray = 0.299f * vec.getX() + 0.587f * vec.getY() + 0.114f * vec.getZ();
        cvImage.at<uchar>(i) = static_cast<uchar>(gray * 255.0f);
    }

    // Initialize QR code detector
    cv::QRCodeDetector qrDetector;
    std::vector<cv::Point2f> points;

    // Detect QR code
    try {
        bool found = qrDetector.detect(cvImage, points);
        if (!found) { // || points.size() != 4) {
            return false;
        }
    } catch (...) {
        return false;
    }

    // Copy corner points to output
    /*
    for (int i = 0; i < 4; i++) {
        outCorners[i].setX(points[i].x);
        outCorners[i].setY(points[i].y);
    }
    */

    return true;
}

bool QRDetection::detectQRFromLuminance(
    const std::vector<uint8_t>& imageBytes,
    int width,
    int height,
    std::vector<std::string>& contents,
    std::vector<Vector2f>& corners)
{
    cv::Mat cvImage(height, width, CV_8U);
    for (int i = 0; i < width * height; i++) {
        cvImage.at<char>(i) = static_cast<char>(imageBytes[i]);
    }

    try {
        cv::QRCodeDetector qrDetector;
        std::vector<cv::Point2f> cornersFound;
        std::string contentsFound = qrDetector.detectAndDecode(cvImage, cornersFound);
        if (!contentsFound.empty()) {
            contents.push_back(contentsFound);

            for (size_t i = 0; i < cornersFound.size(); ++i) {
                cv::Point2f p = cornersFound[i];
                Vector2f corner;
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