#include <iostream>
#include <vector>
#include <opencv2/core.hpp>
#include <opencv2/imgproc.hpp>

#include "Posemesh/BitMatrixTracker/Config.hpp"
#include "Posemesh/BitMatrixTracker/Types.hpp"
#include "Posemesh/BitMatrixTracker/CornerNet.hpp"

namespace psm {
namespace BitMatrixTracker {

// Slide a 5x5 window over a tile ROI (in pixel space), push centers and angles when conf >= threshold.
static inline void scanTileROI(const cv::Mat &normU8,
                               const cv::Rect &roi,
                               float confidenceThreshold,
                               const CornerNetWeights &w,
                               std::vector<cv::Point2f> &outPts,
                               std::vector<float> &outAngles)
{
    // Ensure the 5x5 window stays inside the ROI
    const int xBeg = roi.x + 2;
    const int yBeg = roi.y + 2;
    const int xEnd = roi.x + roi.width  - 3; // inclusive center bound
    const int yEnd = roi.y + roi.height - 3;

    // Fast path for CV_8U
    CV_Assert(normU8.type() == CV_8U);

    uint8_t patch[25];
    for (int y = yBeg; y <= yEnd; ++y) {
        const uint8_t *rowm2 = normU8.ptr<uint8_t>(y - 2);
        const uint8_t *rowm1 = normU8.ptr<uint8_t>(y - 1);
        const uint8_t *row0  = normU8.ptr<uint8_t>(y + 0);
        const uint8_t *rowp1 = normU8.ptr<uint8_t>(y + 1);
        const uint8_t *rowp2 = normU8.ptr<uint8_t>(y + 2);
        for (int x = xBeg; x <= xEnd; ++x) {
            // Manually gather 5x5 without creating submats
            int k = 0;
            const int xm2 = x - 2, xm1 = x - 1, xp1 = x + 1, xp2 = x + 2;
            patch[k++] = rowm2[xm2]; patch[k++] = rowm2[xm1]; patch[k++] = rowm2[x];   patch[k++] = rowm2[xp1]; patch[k++] = rowm2[xp2];
            patch[k++] = rowm1[xm2]; patch[k++] = rowm1[xm1]; patch[k++] = rowm1[x];   patch[k++] = rowm1[xp1]; patch[k++] = rowm1[xp2];
            patch[k++] = row0 [xm2]; patch[k++] = row0 [xm1]; patch[k++] = row0 [x];   patch[k++] = row0 [xp1]; patch[k++] = row0 [xp2];
            patch[k++] = rowp1[xm2]; patch[k++] = rowp1[xm1]; patch[k++] = rowp1[x];   patch[k++] = rowp1[xp1]; patch[k++] = rowp1[xp2];
            patch[k++] = rowp2[xm2]; patch[k++] = rowp2[xm1]; patch[k++] = rowp2[x];   patch[k++] = rowp2[xp1]; patch[k++] = rowp2[xp2];

            float confidence = 0.0f, angle = 0.0f;
            runCornerNet5x5U8(patch, w, confidence, angle);
            if (confidence >= confidenceThreshold) {
                outPts.emplace_back(static_cast<float>(x), static_cast<float>(y));
                // Map angle to [0,90) is already done in runCornerNet*; just store
                outAngles.emplace_back(std::fmod(angle + 180.0f, 180.0f));
            }
        }
    }
}

// Public-facing helper used by Estimator::detectCornersInCluster
// - Only scans tiles that are marked valid in cluster.tileMask
// - Works over a normalized 8-bit image (output of preprocess)
bool detectCornersPerCluster(const cv::Mat &normalizedU8,
                             const Config &cfg,
                             const Cluster &cluster,
                             const CornerNetWeights &weights,
                             Detections &outRaw, // raw before angle grouping/collapse
                             int *rawCountOut)
{
    try {
        if (normalizedU8.empty() || normalizedU8.type() != CV_8U) {
            std::cerr << "detectCornersPerCluster: expected normalized CV_8U image" << std::endl;
            return false;
        }
        outRaw.points.clear();
        outRaw.anglesDeg.clear();

        const int tileSize = cfg.tileSizePx;
        const cv::Rect clusterPx = cluster.pixelBounds & cv::Rect(0, 0, normalizedU8.cols, normalizedU8.rows);
        if (clusterPx.empty())
            return true; // nothing to do

        // Iterate over tiles within cluster.tileBounds
        const int tilesH = cluster.tileMask.rows;
        const int tilesW = cluster.tileMask.cols;
        for (int ty = 0; ty < tilesH; ++ty) {
            for (int tx = 0; tx < tilesW; ++tx) {
                if (!cluster.tileMask(ty, tx))
                    continue;
                // Compute this tile's pixel rect in full image coords
                const int absTileX = cluster.tileBounds.x + tx;
                const int absTileY = cluster.tileBounds.y + ty;
                cv::Rect tilePx(absTileX * tileSize,
                                 absTileY * tileSize,
                                 tileSize,
                                 tileSize);
                // Intersect with cluster pixel bounds (already clamped by caller)
                tilePx &= clusterPx;
                if (tilePx.width < 5 || tilePx.height < 5)
                    continue;

                scanTileROI(normalizedU8, tilePx, cfg.confidenceThreshold, weights,
                            outRaw.points, outRaw.anglesDeg);
            }
        }
        if (rawCountOut) *rawCountOut = static_cast<int>(outRaw.points.size());
        return true;
    } catch (const std::exception &e) {
        std::cerr << "detectCornersPerCluster exception: " << e.what() << std::endl;
        return false;
    }
}

} // namespace BitMatrixTracker
} // namespace psm
