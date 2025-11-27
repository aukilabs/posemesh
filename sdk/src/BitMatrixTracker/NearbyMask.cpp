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
                     std::vector<int16_t> &outNearbyMask)
{
    try {
        outNearbyMask.assign(imgSize.width * imgSize.height, static_cast<int16_t>(-1));

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
                    outNearbyMask[y * W + x] = static_cast<int16_t>(idx);
                }
            }
        }
        return true;
    } catch (const std::exception &e) {
        std::cerr << "buildNearbyMask exception: " << e.what() << std::endl;
        return false;
    }
}

int countInliers(const std::vector<cv::Point2i> &proj,
                 const std::vector<int16_t> &nearbyMask,
                 const int W, const int H)
{
    int inliers = 0;
    for (const auto &p : proj) {
        if (p.x < 0 || p.x >= W || p.y < 0 || p.y >= H)
            continue;

        if (nearbyMask[p.y * W + p.x] >= 0) {
            ++inliers;
        }
    }
    return inliers;

}

static std::vector<bool> s_used; // Reused to avoid memory allocations (for speed!)

// Count inliers for ONE family with one-to-one claiming.
// - proj: pixel projections of target features for this family. (will be rounded to int)
// - label: label map built by buildLabelMap for the same family
int countInliersOneToOne(const std::vector<cv::Point2f> &proj,
                         const std::vector<int16_t> &nearbyMask,
                         const int W, const int H,
                         std::vector<int> &outProjInlierIndices,
                         std::vector<int16_t> &outNearbyMaskInlierLabels)
{
    if (nearbyMask.size() != W * H) {
        std::cerr << "countInliersOneToOne: nearbyMask size mismatch (nearbyMask.size(): " << nearbyMask.size() << ", W: " << W << ", H: " << H << ")" << std::endl;
        return 0;
    }

    outProjInlierIndices.clear();
    outNearbyMaskInlierLabels.clear();

    auto& used = s_used;
    if (used.empty()) {
        // Must fit all detected feature points in the photo.
        // It's fine if too big.
        // Whenever needed we just increase the size below. Keep reusing same buffer to avoid allocations.
        used.resize(1000, false);
    }
    else {
        // Reset the buffer to all false. Then we mark used features to not match same feature multiple times.
        std::fill(used.begin(), used.end(), false);
    }

    int inliers = 0;
    for (int i = 0; i < proj.size(); ++i) {
        const auto &p = proj[i];
        const int px = static_cast<int>(p.x);
        const int py = static_cast<int>(p.y);
        if (px < 0 || px >= W || py < 0 || py >= H)
            continue;

        const int16_t idx = nearbyMask[py * W + px];
        if (idx < 0)
            continue;

        if (idx >= static_cast<int>(used.size())) {
            used.resize(idx + 1000, false); // Expand extra so we don't expand every loop. We never shrink "used" so this would only happen a few times, if ever.
        }
    
        if (used[idx])
            continue; // Don't match same photo feature from different target features.

        used[idx] = true;

        ++inliers;
        outProjInlierIndices.push_back(i);
        outNearbyMaskInlierLabels.push_back(idx);
    }
    return inliers;
}

} // namespace BitMatrixTracker
} // namespace psm
