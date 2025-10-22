#pragma once

#include <opencv2/core.hpp>
#include <random>

namespace psm {
namespace BitMatrixTracker {

std::vector<cv::Point3f> calcObjectSpaceCorners(float physicalSizeMeters);

std::vector<cv::Point2f> calcTargetSpaceCorners(int bitmatrixSideLength); // e.g. 21 for a v1 QR code

bool normalizeVec2(const cv::Vec2d& vec, cv::Vec2d& out, float eps = 1e-9);

void projectWithH(const std::vector<cv::Point2f> &src,
                         const cv::Matx33d &H,
                         std::vector<cv::Point2f> &dst);

cv::Vec2d directionVec(double angleDeg);

double signedAngle2D(const cv::Vec2d &a, const cv::Vec2d &b);

bool homographyFromPointAndDirs(
    const cv::Point2f& markerPoint,
    const cv::Point2f& photoPoint,
    const cv::Vec2d& rightInImage,
    const cv::Vec2d& upInImage,
    int imageWidth,
    int imageHeight,
    double bitMatrixSize,
    const cv::Matx33d& K,
    double sizeFrac,
    cv::Matx33d& homographyOut,
    std::mt19937& rng,
    bool enforceRightHanded = true,
    double eps = 1e-9
);


} // namespace BitMatrixTracker
} // namespace psm