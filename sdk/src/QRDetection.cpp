#include <Posemesh/QRDetection.hpp>
#include <opencv2/objdetect.hpp>
#include <opencv2/imgproc.hpp>

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

} 