#pragma once

#include <vector>
#include <opencv2/core.hpp>

namespace psm {
namespace BitMatrixTracker {

// Compute homography H (3x3) mapping srcPts[i] -> dstPts[i] in a least-squares sense,
// using normalized DLT with SVD, no LM refinement.
// A light-weight alternative to cv::findHomography.
// Returns false if not enough points or degenerate.
bool findHomographyFast(const std::vector<cv::Point2f>& srcPts,
                        const std::vector<cv::Point2f>& dstPts,
                        cv::Mat& outH);

} // namespace BitMatrixTracker
} // namespace psm
