// src/NearbyMask.cpp
#include <vector>
#include <limits>
#include <iostream>
#include <opencv2/core.hpp>

namespace psm {
namespace BitMatrixTracker {

// Cached disk offsets for a given integer radius
static std::vector<cv::Point> &diskOffsets(int radius)
{
    static int cachedRadius = -1;
    static std::vector<cv::Point> cache;
    if (radius == cachedRadius && !cache.empty())
        return cache;

    cachedRadius = radius;
    cache.clear();
    const int r = std::max(0, radius);
    const int r2 = r * r;
    cache.reserve((2*r+1)*(2*r+1));
    for (int dy = -r; dy <= r; ++dy) {
        for (int dx = -r; dx <= r; ++dx) {
            if (dx*dx + dy*dy <= r2)
                cache.emplace_back(dx, dy);
        }
    }
    return cache;
}

// Build a label map for a set of collapsed detection centers.
// - imgSize: full image size (we allocate full-sized label to keep lookups branch-free)
// - centers: detection centers for ONE family (already collapsed/deduped)
// - radiusPx: inlier radius (e.g., 6.0f)
// - outLabel: CV_16S, -1 for no detection, otherwise detection index [0..centers.size()-1]
bool buildLabelMap(const cv::Size &imgSize,
                   const std::vector<cv::Point2f> &centers,
                   float radiusPx,
                   cv::Mat1s &outLabel)
{
    try {
        outLabel.create(imgSize);
        outLabel = -1;

        if (centers.empty())
            return true;

        // Temporary best distance^2 per pixel (uint16 max is plenty since radius<=~255)
        cv::Mat bestDist(imgSize, CV_16U, cv::Scalar(std::numeric_limits<uint16_t>::max()));

        const int r = std::max(0, static_cast<int>(std::round(radiusPx)));
        auto &offs = diskOffsets(r);

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
                uint16_t &cur = bestDist.at<uint16_t>(y, x);
                if (d2 < cur) {
                    cur = d2;
                    outLabel(y, x) = static_cast<int16_t>(idx);
                }
            }
        }
        return true;
    } catch (const std::exception &e) {
        std::cerr << "buildLabelMap exception: " << e.what() << std::endl;
        return false;
    }
}

// Count inliers for ONE family with one-to-one claiming.
// - proj: integer pixel projections of target features for this family
// - label: label map built by buildLabelMap for the same family
int countInliersOneToOne(const std::vector<cv::Point2i> &proj,
                         const cv::Mat1s &label)
{
    const int H = label.rows, W = label.cols;
    // Compute max index present in label to size the used[] bitset compactly
    int16_t maxIdx = -1;
    {
        // Light scan over proj to avoid scanning entire label
        for (const auto &p : proj) {
            if ((unsigned)p.x >= (unsigned)W || (unsigned)p.y >= (unsigned)H) continue;
            int16_t idx = label(p.y, p.x);
            if (idx > maxIdx) maxIdx = idx;
        }
    }
    int inliers = 0;
    if (maxIdx < 0) {
        // No valid indices touched by these projections
        return 0;
    }
    std::vector<uint8_t> used(static_cast<size_t>(maxIdx) + 1, 0);

    for (const auto &p : proj) {
        if ((unsigned)p.x >= (unsigned)W || (unsigned)p.y >= (unsigned)H)
            continue;
        int16_t idx = label(p.y, p.x);
        if (idx < 0) continue;
        if (idx < static_cast<int>(used.size())) {
            if (used[idx]) continue;
            used[idx] = 1;
        }
        ++inliers;
    }
    return inliers;
}

} // namespace BitMatrixTracker
} // namespace psm
