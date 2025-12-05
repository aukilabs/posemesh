#include <iostream>
#include <iomanip>
#include <chrono>
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

// Forward declared from NearbyMask.cpp
namespace psm {
namespace BitMatrixTracker {
bool buildNearbyMask(const cv::Size &imgSize,
    const std::vector<cv::Point2f> &centers,
    float radiusPx,
    std::vector<int16_t> &outNearbyMask);
}
}

void drawCornerOutlineOnPhoto(const cv::Mat &rgb, const std::vector<cv::Point2f> &corners, cv::Scalar color)
{
    std::cout << "corners: " << corners.size() << std::endl;
    for (const auto &corner : corners) {
        std::cout << "-- " << corner.x << ", " << corner.y << std::endl;
    }

    std::vector<cv::Point2i> cornersInt(4);
    for (int i = 0; i < corners.size(); i++) {
        cornersInt[i] = cv::Point2i(std::round(corners[i].x), std::round(corners[i].y));
    }

    std::vector<std::vector<cv::Point2i>> contours = { cornersInt };
    cv::drawContours(rgb, contours, 0, color, 1);
    for (int i = 0; i < corners.size(); i++) {
        const auto &corner = corners[i];
        cv::circle(rgb, corner, 4, color);
    }
    cv::circle(rgb, corners[0], 6, color, 1); // Make first corner extra big

    // Axis lines
    cv::line(rgb, corners[0], (corners[0]*2+corners[1])/3, cv::Scalar(0, 0, 255), 2);
    cv::line(rgb, corners[0], (corners[0]*2+corners[3])/3, cv::Scalar(255, 0, 0), 2);
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
                              const cv::Scalar &color, int pointRadius=4)
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
        cv::circle(rgb, feat, pointRadius, color);
    }
}

void drawTargetOnPhoto(const cv::Mat &rgb, const Target &target, 
                             const cv::Vec3f &rvec, const cv::Vec3f &tvec, const cv::Matx33f &K,
                             const cv::Scalar &color1, const cv::Scalar &color2, int pointRadius=4)
{
    drawPoseOutlineOnPhoto(rgb, target.sideLengthMeters, rvec, tvec, K, color1);
    drawFeaturePointsOnPhoto(rgb, target.diag1, target.bitmatrix.cols, target.sideLengthMeters, rvec, tvec, K, color1, pointRadius);
    drawFeaturePointsOnPhoto(rgb, target.diag2, target.bitmatrix.cols, target.sideLengthMeters, rvec, tvec, K, color2, pointRadius);
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
    //{ "424272.304214.jpg",1432.85,1432.85,961.3515,725.1973,0.1f},
    { "424273.004447.jpg",1434.154,1434.154,960.8257,724.7334,0.1f},
    { "424274.404912.jpg",1436.709,1436.709,959.9976,725.895,0.1f},
    //{ "424243.594662.jpg",1434.544,1434.544,961.1267,724.8087,0.1f},
    { "424244.394929.jpg",1435.665,1435.665,960.1682,724.1594,0.1f},
};

bool processExampleFrame(const ExampleFrame &exampleFrame, Estimator& estimator, float& outRuntimeMs, float& outPoseErrorCm, float& outAngleErrorDeg, Diagnostics* outDiagnostics=nullptr)
{
    bool verbose = outDiagnostics != nullptr;

    std::string imagePath = framesFolder + "/" + exampleFrame.jpgName;

    cv::Mat rgb; // Only used for saving plots, when verbose is true
    cv::Mat gray;
    if (verbose) {
        // Load with RGB for plotting
        std::cout << "Loading RGB image from: " << imagePath << std::endl;
        rgb = cv::imread(imagePath, cv::IMREAD_COLOR);
        cv::cvtColor(rgb, gray, cv::COLOR_BGR2GRAY);
    }
    else {
        // Load only grayscale (enough for detection)
        gray = cv::imread(imagePath, cv::IMREAD_GRAYSCALE);
    }

    if (gray.empty()) {
        std::cerr << "Could not load image: " << imagePath << std::endl;
        return false;
    }

    if (verbose) {
        std::cout << "rgb size: " << rgb.size() << std::endl;
        std::cout << "gray size: " << gray.size() << std::endl;
        std::cout << "gray type: " << gray.type() << std::endl;
        cv::imwrite("rgbPhoto.jpg", rgb);
        cv::imwrite("grayPhoto.jpg", gray);
    }

    Target target;

    // Detect QR code in the image using OpenCV, extract its bit matrix, and build the Target.
    std::vector<cv::Point2f> corners;
    cv::Mat qrStraight;
    cv::QRCodeDetector qrDecoder;

    // Detect and decode the QR code
    const std::string decodedInfo = qrDecoder.detectAndDecode(gray, corners, qrStraight);
    if (verbose) {
        std::cout << "Decoded QR contents: " << decodedInfo << std::endl;
        std::cout << "corners: " << corners.size() << std::endl;
        for (const auto &corner : corners) {
            std::cout << "-- " << corner.x << ", " << corner.y << std::endl;
        }
        std::cout << "qrStraight.size(): " << qrStraight.size() << std::endl;
    }

    if (decodedInfo.empty() || corners.size() != 4 || qrStraight.empty()) {
        std::cerr << "OpenCV comparison failed to detect or decode QR code from image" << std::endl;
        return false;
    }

    if (verbose) {
        cv::Mat plot = rgb.clone();
        drawCornerOutlineOnPhoto(plot, corners);
        cv::imwrite("poseOpenCV.jpg", plot);
    }

    // Binarize the straightened QR code image to get the bit matrix
    cv::Mat1b bitmatrix;
    cv::threshold(qrStraight, bitmatrix, 0, 1, cv::THRESH_BINARY | cv::THRESH_OTSU);

    if (bitmatrix.type() != CV_8U) {
        bitmatrix.convertTo(bitmatrix, CV_8U);
    }

    if (verbose) {
        std::cout << "bitmatrix size: " << bitmatrix.size() << std::endl;
        cv::Mat bitmatrix_img;
        bitmatrix.convertTo(bitmatrix_img, CV_8U, 255);
        cv::resize(bitmatrix_img, bitmatrix_img, cv::Size(bitmatrix.cols * 10, bitmatrix.rows * 10), 0, 0, cv::INTER_NEAREST);
        cv::imwrite("bitmatrix.jpg", bitmatrix_img);
    }

    if (!makeTargetFromBitmatrix(bitmatrix, exampleFrame.physicalSizeMeters, target)) {
        std::cerr << "Failed to build target from detected QR code" << std::endl;
        return false;
    }


    // TODO move to Target.cpp file
    //target.buildNearbyMasks();
    std::vector<cv::Point2f> targetDiag1, targetDiag2;
    targetDiag1.reserve(target.diag1.size());
    targetDiag2.reserve(target.diag2.size());
    for (const auto& p : target.diag1) {
        targetDiag1.emplace_back(p.x, p.y);
    }
    for (const auto& p : target.diag2) {
        targetDiag2.emplace_back(p.x, p.y);
    }
    if (!buildNearbyMask(target.bitmatrix.size(), targetDiag1, 0.5f, target.nearbyMask1)) {
        std::cerr << "buildNearbyMask target.diag1 failed" << std::endl;
        return false;
    }

    if (!buildNearbyMask(target.bitmatrix.size(), targetDiag2, 0.5f, target.nearbyMask2)) {
        std::cerr << "buildNearbyMask target.diag2 failed" << std::endl;
        return false;
    }

    // Camera intrinsics (fx, fy, cx, cy). Only supporting pinhole for now. Image should be rectified already. Distorted photos not tested.
    cv::Matx33d K(exampleFrame.fx, 0.0, exampleFrame.cx,
                  0.0, exampleFrame.fy, exampleFrame.cy,
                  0.0, 0.0, 1.0);
    if (verbose) {
        std::cout << "K: " << K << std::endl;
    }

    // Compare with pose from openCV corners, for reference. In real use we won't run this.
    std::vector<cv::Point3f> objectCornersPoint3d = calcObjectSpaceCorners(exampleFrame.physicalSizeMeters);
    cv::Vec3d rvecTruth, tvecTruth;
    bool gotPoseTruth = cv::solvePnP(objectCornersPoint3d, corners, K, cv::noArray(), rvecTruth, tvecTruth, false, cv::SOLVEPNP_SQPNP);
    if (verbose) {
        if (gotPoseTruth) {
            std::cout << "rvecTruth: " << rvecTruth.t() << std::endl;
            std::cout << "tvecTruth: " << tvecTruth.t() << std::endl;
        }
        else {
            std::cout << "solvePnP failed for truth pose comparison" << std::endl;
        }
    }

    Pose pose;
    cv::Matx33d H;

    auto startTime = std::chrono::high_resolution_clock::now();
    try {
        if (!estimator.estimatePose(gray, K, target, pose, H, outDiagnostics)) {
            std::cerr << "Pose estimation failed" << std::endl;
            return false;
        }
    } catch (const std::exception &e) {
        std::cerr << "Pose estimation failed with exception: " << e.what() << std::endl;
        return false;
    }
    auto endTime = std::chrono::high_resolution_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::microseconds>(endTime - startTime).count();
    std::cout << "Finding pose took: " << std::fixed << std::setprecision(3) << (duration / 1000.0) << " ms" << std::endl;

    double tvecError = cv::norm(pose.tvec - tvecTruth);
    double rvecAngleError = rvecAngleDelta(rvecTruth, pose.rvec);


    outPoseErrorCm = tvecError * 100.0;
    outAngleErrorDeg = rvecAngleError;
    outRuntimeMs = duration / 1000.0;

    if (verbose) {
        std::cout << "inliers: " << outDiagnostics->inliersBest << ", iters: " << outDiagnostics->ransacIterations << "\n";

        double rvecError = cv::norm(pose.rvec - rvecTruth);

        std::cout << "truth rvec: " << rvecTruth.t() << "\n";
        std::cout << "truth tvec: " << tvecTruth.t() << "\n";
        std::cout << "rvec: " << pose.rvec.t() << "\n";
        std::cout << "tvec: " << pose.tvec.t() << "\n";
        std::cout << "tvec error: " << tvecError << "\n";
        std::cout << "rvec error: " << rvecError << "\n";
        std::cout << "rvec angle error: " << rvecAngleError << "\n";

        // Draw truth pose (in green) and detected pose (in blue) on the input photo and save as "poseComparison.jpg".
        // Assume drawPoseOnPhoto(cv::Mat &img, const cv::Matx33d &K, const cv::Vec3d &rvec, const cv::Vec3d &tvec, float sideLengthMeters, const cv::Scalar &color, int thickness)
        // - If available, otherwise adapt call.
        cv::Mat comparisonPlot = rgb.clone();
        drawTargetOnPhoto(comparisonPlot, target, rvecTruth, tvecTruth, K, cv::Scalar(0, 255, 0), cv::Scalar(150, 200, 0), 3);
        drawTargetOnPhoto(comparisonPlot, target, pose.rvec, pose.tvec, K, cv::Scalar(255, 0, 255), cv::Scalar(50, 100, 255), 4);
        cv::imwrite("poseComparison.jpg", comparisonPlot);
    }
    else {
        std::cout << "Pose error: " << std::fixed << std::setprecision(3) << (tvecError * 100.0) << " cm, " << rvecAngleError << "°" << std::endl;
    }

    return true;
}

void printAggregatedStats(const std::string &title, const std::vector<float> &values)
{
    if (values.empty()) {
        std::cout << title << ": <no values>" << std::endl;
        return;
    }

    float avg = std::accumulate(values.begin(), values.end(), 0.0f) / values.size();
    float max = *std::max_element(values.begin(), values.end());
    std::cout << title << ": Avg " << std::fixed << std::setprecision(3) << avg << ", max=" << max << std::endl;
}

int main(int argc, char *argv[])
{
    Config cfg = defaultConfig();
    cfg.cornerNetWeightsPath = "cornernet_2025-10-12_1.bin";
    cfg.ransacMaxIters = 30000;
    cfg.maxInnerRefinements = 0;
    cfg.angleJitterDeg = 0.1f;
    cfg.inlierRadiusPx = 4.0f;
    cfg.earlyStopPercent = 70;
    cfg.collapseRadiusPx = 3.0f;
    cfg.useFindHomographyFast = true;
    cfg.finalRefinePnP = true;

    bool verbose = false;
    bool evalMode = false;
    for (int i = 1; i < argc; ++i) {
        std::string param = argv[i];
        if (param == "-v" || param == "--verbose") {
            verbose = true;
            break;
        }
        else if (param == "--eval") {
            evalMode = true;
            break;
        }
    }

    if (exampleFrames.empty()) {
        std::cerr << "No example frames" << std::endl;
        return 1;
    }

    Estimator estimator(cfg);

    int successfulCount = 0;
    std::vector<float> runtimes, positionErrors, angleErrors;

    if (argc >= 2 && !evalMode) {
        std::string frameIndexStr = argv[1];
        int frameIndex;
        if (std::istringstream(frameIndexStr) >> frameIndex) {
            if (frameIndex >= 0 && frameIndex < exampleFrames.size()) {
                std::cout << "Processing ONLY example frame " << (frameIndex) << ": " << exampleFrames[frameIndex].jpgName << std::endl;
                float runtimeMs, poseErrorCm, angleErrorDeg;
                processExampleFrame(exampleFrames[frameIndex], estimator, runtimeMs, poseErrorCm, angleErrorDeg);
                return 0;
            }
            else {
                std::cerr << "Invalid frame index: " << frameIndex << std::endl;
                return 1;
            }
        }
        else {
            std::cerr << "Parameter not a valid frame index: " << frameIndexStr << std::endl;
            return 1;
        }
    }

    const int repeats = evalMode ? 20 : 1;
    for (int r = 0; r < repeats; r++) {
        for (int i = 0; i < exampleFrames.size(); i++) {
            std::cout << std::endl;
            std::cout << "Processing example frame " << (i) << ": " << exampleFrames[i].jpgName << std::endl;
            float runtimeMs, poseErrorCm, angleErrorDeg;
            Diagnostics diagnostics;
            Diagnostics* diagPtr = verbose ? &diagnostics : nullptr;
            bool success = processExampleFrame(exampleFrames[i], estimator, runtimeMs, poseErrorCm, angleErrorDeg, diagPtr);
            if (success) {
                successfulCount++;
                runtimes.push_back(runtimeMs);
                positionErrors.push_back(poseErrorCm);
                angleErrors.push_back(angleErrorDeg);
            }
        }
    }
    
    if (evalMode) {
        printAggregatedStats("Time (ms)", runtimes);
        printAggregatedStats("Position Error (cm)", positionErrors);
        printAggregatedStats("Angle Error (deg)", angleErrors);
    }

    return 0;
}