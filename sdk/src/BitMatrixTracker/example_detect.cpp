#include <iostream>
#include <opencv2/core.hpp>
#include <opencv2/imgcodecs.hpp>
#include <opencv2/objdetect.hpp>
#include <opencv2/imgproc.hpp>
#include <opencv2/calib3d.hpp>
#include "Posemesh/Vector2.hpp"
#include "Posemesh/QRDetection.hpp"
#include "Posemesh/BitMatrixTracker/Estimator.hpp"
#include "Posemesh/BitMatrixTracker/Precompute.hpp"
#include "Posemesh/BitMatrixTracker/Geometry.hpp"


using namespace psm::BitMatrixTracker;

void drawCornerOutlineOnPhoto(const cv::Mat &rgb, const std::vector<cv::Point2f> &corners, cv::Scalar color)
{
    std::cout << "corners: " << corners.size() << std::endl;
    for (const auto &corner : corners) {
        std::cout << "-- " << corner.x << ", " << corner.y << std::endl;
    }

    std::vector<cv::Point2i> cornersInt(4);
    for (int i = 0; i < corners.size(); i++) {
        cornersInt[i] = cv::Point2i(static_cast<int>(corners[i].x), static_cast<int>(corners[i].y));
    }

    std::vector<std::vector<cv::Point2i>> contours = { cornersInt };
    cv::drawContours(rgb, contours, 0, color, 1);
    for (int i = 0; i < corners.size(); i++) {
        const auto &corner = corners[i];
        cv::circle(rgb, corner, 4, color);
    }
}

void drawCornerOutlineOnPhoto(const cv::Mat &rgb, const std::vector<cv::Point2f> &corners)
{
    drawCornerOutlineOnPhoto(rgb, corners, cv::Scalar(0, 255, 0));
}

void drawPoseOutlineOnPhoto(const cv::Mat &rgb, float sideLengthMeters, const cv::Vec3f &rvec, const cv::Vec3f &tvec, const cv::Matx33f &K, const cv::Scalar &color)
{
    std::vector<cv::Point3f> objCorners = calcObjectSpaceCorners(sideLengthMeters);
    std::vector<cv::Point2f> projectedCorners;
    cv::projectPoints(objCorners, rvec, tvec, K, cv::noArray(), projectedCorners);

    drawCornerOutlineOnPhoto(rgb, projectedCorners, color);
}

void drawFeaturePointsOnPhoto(const cv::Mat &rgb, const std::vector<cv::Point2f> &diagFeats,
                              int bitmatrixSideLength, float physicalSizeMeters,
                              const cv::Vec3f &rvec, const cv::Vec3f &tvec, const cv::Matx33f &K,
                              const cv::Scalar &color)
{
    std::vector<cv::Point3f> objFeats;

    // Bitmatrix space to object space
    for (const auto &feat : diagFeats) {
        objFeats.push_back(cv::Point3f(
            (-0.5f + feat.x / bitmatrixSideLength) * physicalSizeMeters,
            (-0.5f + feat.y / bitmatrixSideLength) * physicalSizeMeters, 
            .0f)
        );
    }
    std::vector<cv::Point2f> projectedFeats;
    cv::projectPoints(objFeats, rvec, tvec, K, cv::noArray(), projectedFeats);
    for (const auto &feat : projectedFeats) {
        cv::circle(rgb, feat, 4, color);
    }
}

void drawTargetOnPhoto(const cv::Mat &rgb, const Target &target, 
                             const cv::Vec3f &rvec, const cv::Vec3f &tvec, const cv::Matx33f &K,
                             const cv::Scalar &color1, const cv::Scalar &color2)
{
    drawPoseOutlineOnPhoto(rgb, target.sideLengthMeters, rvec, tvec, K, color1);
    drawFeaturePointsOnPhoto(rgb, target.diag1, target.bitmatrix.cols, target.sideLengthMeters, rvec, tvec, K, color1);
    drawFeaturePointsOnPhoto(rgb, target.diag2, target.bitmatrix.cols, target.sideLengthMeters, rvec, tvec, K, color2);
}

struct ExampleFrame {
    std::string jpgName;
    float fx;
    float fy;
    float cx;
    float cy;
    float physicalSizeMeters;
};

const std::string framesFolder = "scannertest/dmt_scan_2024-06-26_14-08-53/Frames";
std::vector<ExampleFrame> exampleFrames = {
    {
        "424274.604979.jpg", 1436.762, 1436.762, 960.2018, 725.8228, 0.1f
    }
};

int main(int argc, char *argv[])
{
    Config cfg = defaultConfig();
    cfg.cornerNetWeightsPath = "cornernet_2025-10-12_1.bin";
    cfg.ransacMaxIters = 500000;
    cfg.inlierRadiusPx = 5;

    Estimator estimator(cfg);

    if (exampleFrames.empty()) {
        std::cerr << "No example frames" << std::endl;
        return 1;
    }

    const ExampleFrame &exampleFrame = exampleFrames[0];
    std::string imagePath = framesFolder + "/" + exampleFrame.jpgName;

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
    std::vector<cv::Point2f> corners;
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
    drawCornerOutlineOnPhoto(plot, corners);
    cv::imwrite("poseOpenCV.jpg", plot);

    // Binarize the straightened QR code image to get the bit matrix
    cv::Mat1b bitmatrix;
    cv::threshold(qr_straight, bitmatrix, 0, 1, cv::THRESH_BINARY | cv::THRESH_OTSU);

    // Optionally, resize to a canonical size if needed (OpenCV's output is usually square)
    // For robustness, ensure bitmatrix is CV_8U and values are 0/1
    if (bitmatrix.type() != CV_8U) {
        bitmatrix.convertTo(bitmatrix, CV_8U);
    }

    //cv::flip(bitmatrix, bitmatrix, 1);

    std::cout << "bitmatrix size: " << bitmatrix.size() << std::endl;
    cv::Mat bitmatrix_img;
    bitmatrix.convertTo(bitmatrix_img, CV_8U, 255);
    cv::resize(bitmatrix_img, bitmatrix_img, cv::Size(bitmatrix.cols * 10, bitmatrix.rows * 10), 0, 0, cv::INTER_NEAREST);
    cv::imwrite("bitmatrix.jpg", bitmatrix_img);

    if (!makeTargetFromBitmatrix(bitmatrix, exampleFrame.physicalSizeMeters, target)) {
        std::cerr << "Failed to build target from detected QR code" << std::endl;
        return 1;
    }

    // Camera intrinsics (fx, fy, cx, cy)
    cv::Matx33d K(exampleFrame.fx, 0.0, exampleFrame.cx,
                  0.0, exampleFrame.fy, exampleFrame.cy,
                  0.0, 0.0, 1.0);
    std::cout << "K: " << K << std::endl;

    // Compare with pose from openCV corners, for reference. In real use we won't run this.
    const double halfSide = exampleFrame.physicalSizeMeters / 2.0;
    std::vector<cv::Point3f> objectCornersPoint3d = calcObjectSpaceCorners(exampleFrame.physicalSizeMeters);
    cv::Vec3d rvecTruth, tvecTruth;
    bool gotPoseTruth = cv::solvePnP(objectCornersPoint3d, corners, K, cv::noArray(), rvecTruth, tvecTruth, false, cv::SOLVEPNP_ITERATIVE);
    if (gotPoseTruth) {
        std::cout << "rvecTruth: " << rvecTruth.t() << std::endl;
        std::cout << "tvecTruth: " << tvecTruth.t() << std::endl;
    }
    else {
        std::cout << "solvePnP failed for truth pose comparison" << std::endl;
    }

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

    std::cout << "truth rvec: " << rvecTruth.t() << "\n";
    std::cout << "truth tvec: " << tvecTruth.t() << "\n";
    std::cout << "rvec: " << pose.rvec.t() << "\n";
    std::cout << "tvec: " << pose.tvec.t() << "\n";
    double tvecError = cv::norm(pose.tvec - tvecTruth);
    std::cout << "tvec error: " << tvecError << "\n";
    double rvecError = cv::norm(pose.rvec - rvecTruth);
    std::cout << "rvec error: " << rvecError << "\n";

    std::cout << "inliers: " << diag.inliersBest << ", iters: " << diag.ransacIterations << "\n";

    // INSERT_YOUR_CODE

    // Draw truth pose (in green) and detected pose (in blue) on the input photo and save as "poseComparison.jpg".
    // Assume drawPoseOnPhoto(cv::Mat &img, const cv::Matx33d &K, const cv::Vec3d &rvec, const cv::Vec3d &tvec, float sideLengthMeters, const cv::Scalar &color, int thickness)
    // - If available, otherwise adapt call.
    cv::Mat comparisonPlot = rgb.clone();
    drawTargetOnPhoto(comparisonPlot, target, rvecTruth, tvecTruth, K, cv::Scalar(0, 255, 0), cv::Scalar(0, 200, 50));
    drawTargetOnPhoto(comparisonPlot, target, pose.rvec, pose.tvec, K, cv::Scalar(255, 0, 255), cv::Scalar(255, 0, 200));

    cv::imwrite("poseComparison.jpg", comparisonPlot);

    return 0;
}