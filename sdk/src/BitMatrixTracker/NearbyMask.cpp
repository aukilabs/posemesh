// src/NearbyMask.cpp
#include <vector>
#include <limits>
#include <iostream>
#include <opencv2/core.hpp>

namespace psm {
namespace BitMatrixTracker {

// Cached disk offsets for a given integer radius
static std::vector<cv::Point> &diskOffsets(float radius)
{
    static float cachedRadius = -1.0f;
    static std::vector<cv::Point> cache;
    if (radius == cachedRadius && !cache.empty())
        return cache;

    cachedRadius = radius;
    cache.clear();
    const float r2 = radius * radius;
    const int range = (int)(std::ceilf(radius));
    cache.reserve((2*range+1)*(2*range+1));
    for (int dy = -range; dy <= range; ++dy) {
        for (int dx = -range; dx <= range; ++dx) {
            if (dx*dx + dy*dy <= r2)
                cache.emplace_back(dx, dy);
        }
    }
    return cache;
}

// Build a nearby mask for a set of collapsed detection centers.
// - imgSize: full image size (we allocate full-sized label to keep lookups branch-free)
// - centers: detection centers for ONE family (already collapsed/deduped)
// - radiusPx: inlier radius (e.g., 6.0f)
// - outNearbyMask: CV_16S, -1 for no detection, otherwise detection index [0..centers.size()-1]
bool buildNearbyMask(const cv::Size &imgSize,
                     const std::vector<cv::Point2f> &centers,
                     float radiusPx,
                     cv::Mat1s &outNearbyMask)
{
    try {
        outNearbyMask.create(imgSize);
        outNearbyMask = -1;

        if (centers.empty())
            return true;

        // Temporary best distance^2 per pixel (uint16 max is plenty since radius<=~255)
        //cv::Mat bestDist(imgSize, CV_16U, cv::Scalar(std::numeric_limits<uint16_t>::max()));
        std::vector<uint16_t> bestDist(imgSize.width * imgSize.height, std::numeric_limits<uint16_t>::max());

        auto &offs = diskOffsets(radiusPx);

        const int H = imgSize.height;
        const int W = imgSize.width;

        for (int idx = 0; idx < static_cast<int>(centers.size()); ++idx) {
            const int cx = static_cast<int>(std::round(centers[idx].x));
            const int cy = static_cast<int>(std::round(centers[idx].y));
            for (const cv::Point &o : offs) {
                const int x = cx + o.x;
                const int y = cy + o.y;
                if ((unsigned)x >= (unsigned)W || (unsigned)y >= (unsigned)H)
                    continue;
                // d2 = o.x^2 + o.y^2 (precomputed implicitly)
                const uint16_t d2 = static_cast<uint16_t>(o.x*o.x + o.y*o.y);
                //uint16_t &cur = bestDist.at<uint16_t>(y, x);
                uint16_t &cur = bestDist[y * W + x];
                if (d2 < cur) {
                    cur = d2;
                    outNearbyMask(y, x) = static_cast<int16_t>(idx);
                }
            }
        }
        return true;
    } catch (const std::exception &e) {
        std::cerr << "buildNearbyMask exception: " << e.what() << std::endl;
        return false;
    }
}

// Count inliers for ONE family with one-to-one claiming.
// - proj: pixel projections of target features for this family. (will be rounded to int)
// - label: label map built by buildLabelMap for the same family
int countInliersOneToOne(const std::vector<cv::Point2f> &proj,
                         const cv::Mat1s &nearbyMask,
                         std::vector<int> &outProjInlierIndices,
                         std::vector<int16_t> &outNearbyMaskInlierLabels)
{
    const int H = nearbyMask.rows;
    const int W = nearbyMask.cols;

    outProjInlierIndices.clear();
    outNearbyMaskInlierLabels.clear();

    // Compute max index present in label to size the used[] bitset compactly
    int16_t maxIdx = -1;
    {
        // Light scan over proj to avoid scanning entire label
        for (const auto &p : proj) {
            int x = static_cast<int>(p.x);
            int y = static_cast<int>(p.y);
            if ((unsigned)x >= (unsigned)W || (unsigned)y >= (unsigned)H)
                continue;
            const int16_t &idx = nearbyMask(y, x);
            if (idx > maxIdx)
                maxIdx = idx;
        }
    }
    int inliers = 0;
    if (maxIdx < 0) {
        // No valid indices touched by these projections
        return 0;
    }

    std::vector<uint8_t> used(static_cast<size_t>(maxIdx) + 1, 0);

    for (int i = 0; i < proj.size(); ++i) {
        const auto &p = proj[i];
        if (p.x >= W || p.y >= H || p.x < 0 || p.y < 0)
            continue;

        int16_t idx = nearbyMask(p.y, p.x);
        if (idx < 0)
            continue;

        if (idx < static_cast<int>(used.size())) {
            if (used[idx])
                continue;
            used[idx] = 1;
        }
        ++inliers;
        outProjInlierIndices.push_back(i);
        outNearbyMaskInlierLabels.push_back(idx);
    }
    return inliers;
}

} // namespace BitMatrixTracker
} // namespace psm
