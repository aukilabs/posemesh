#include <iostream>
#include <opencv2/core.hpp>
#include <opencv2/imgcodecs.hpp>
#include <opencv2/objdetect.hpp>
#include <opencv2/imgproc.hpp>
#include "Posemesh/Vector2.hpp"
#include "Posemesh/QRDetection.hpp"
#include "Estimator.hpp"
#include "Precompute.hpp"

void drawPoseOnPhoto(const cv::Mat &rgb, const std::vector<cv::Point> &corners)
{
    std::vector<std::vector<cv::Point>> contours;
    contours.push_back(corners);
    cv::drawContours(rgb, contours, 0, cv::Scalar(0, 255, 0), 1);
    for (int i = 0; i < corners.size(); i++) {
        const auto &corner = corners[i];
        cv::circle(rgb, corner, 4, cv::Scalar(0, 255, 0));
    }
}

int main(int argc, char *argv[])
{
    using namespace psm::BitMatrixTracker;

    Config cfg = defaultConfig();
    //cfg.validateSubtiles = false;
    cfg.peakThreshold = 0.25f; 
    //cfg.minContrast = 0.1f;

    Estimator estimator(cfg);

    std::string imagePath =
        argc > 1 ?
        argv[1] :
        "scannertest/dmt_scan_2024-06-26_14-08-53/Frames/424274.604979.jpg";

    // Load grayscale, undistorted 1920x1080
    std::cout << "imagePath: " << imagePath << std::endl;
    cv::Mat rgb = cv::imread(imagePath, cv::IMREAD_COLOR);
    cv::Mat gray;
    cv::cvtColor(rgb, gray, cv::COLOR_BGR2GRAY);
    if (gray.empty()) {
        std::cerr << "Could not load image: " << imagePath << std::endl;
        return 1;
    }

    std::cout << "rgb size: " << rgb.size() << std::endl;
    std::cout << "gray size: " << gray.size() << std::endl;
    std::cout << "gray type: " << gray.type() << std::endl;
    cv::imwrite("rgbPhoto.jpg", rgb);
    cv::imwrite("grayPhoto.jpg", gray);

    Target target;
    // Detect QR code in the image using OpenCV, extract its bit matrix, and build the Target.
    std::vector<cv::Point> corners;
    std::string decoded_info;
    cv::Mat qr_straight;
    cv::QRCodeDetector qrDecoder;

    // Detect and decode the QR code
    decoded_info = qrDecoder.detectAndDecode(gray, corners, qr_straight);
    std::cout << "decoded_info: " << decoded_info << std::endl;
    std::cout << "corners: " << corners.size() << std::endl;
    for (const auto &corner : corners) {
        std::cout << "-- " << corner.x << ", " << corner.y << std::endl;
    }
    std::cout << "qr_straight: " << qr_straight.size() << std::endl;

    if (decoded_info.empty() || corners.size() != 4 || qr_straight.empty()) {
        std::cerr << "Failed to detect or decode QR code from image" << std::endl;
        return 1;
    }

    cv::Mat plot = rgb.clone();
    drawPoseOnPhoto(plot, corners);
    cv::imwrite("poseOpenCV.jpg", plot);

    // Binarize the straightened QR code image to get the bit matrix
    cv::Mat1b bitmatrix;
    cv::threshold(qr_straight, bitmatrix, 0, 1, cv::THRESH_BINARY | cv::THRESH_OTSU);

    // Optionally, resize to a canonical size if needed (OpenCV's output is usually square)
    // For robustness, ensure bitmatrix is CV_8U and values are 0/1
    if (bitmatrix.type() != CV_8U) {
        bitmatrix.convertTo(bitmatrix, CV_8U);
    }

    std::cout << "bitmatrix size: " << bitmatrix.size() << std::endl;
    cv::Mat bitmatrix_img = bitmatrix.clone();
    cv::resize(bitmatrix, bitmatrix_img, cv::Size(bitmatrix.cols * 5, bitmatrix.rows * 5), 0, 0, cv::INTER_NEAREST);
    cv::imwrite("bitmatrix.jpg", bitmatrix_img);


    const float physicalSizeMeters = 0.10f;
    if (!makeTargetFromBitmatrix(bitmatrix, physicalSizeMeters, target)) {
        std::cerr << "Failed to build target from detected QR code" << std::endl;
        return 1;
    }

    // Camera intrinsics (fx, fy, cx, cy)
    // 1436.762,1436.762,960.2018,725.8228
    const float fx = 1436.762;
    const float fy = 1436.762;
    const float cx = 960.2018;
    const float cy = 725.8228;
    cv::Matx33d K(fx, 0.0, cx,
                  0.0, fy, cy,
                  0.0, 0.0, 1.0);
    std::cout << "K: " << K << std::endl;

    Pose pose;
    cv::Matx33d H;
    Diagnostics diag;

    try {
        if (!estimator.estimatePose(gray, K, target, pose, H, &diag)) {
            std::cerr << "Pose estimation failed" << std::endl;
            return 1;
        }
    } catch (const std::exception &e) {
        std::cerr << "Pose estimation failed: " << e.what() << std::endl;
        return 1;
    }

    std::cout << "rvec: " << pose.rvec.t() << "\n";
    std::cout << "tvec: " << pose.tvec.t() << "\n";
    std::cout << "inliers: " << diag.inliersBest << ", iters: " << diag.ransacIterations << "\n";

    return 0;
}