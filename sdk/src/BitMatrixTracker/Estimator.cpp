#include <iostream>
#include <memory>
#include <algorithm>
#include <numeric>
#include <opencv2/imgcodecs.hpp>
#include <opencv2/imgproc.hpp>
#include <opencv2/calib3d.hpp>
#include "Posemesh/BitMatrixTracker/Estimator.hpp"
#include "Posemesh/BitMatrixTracker/CornerNet.hpp"
#include "Posemesh/BitMatrixTracker/Geometry.hpp"

namespace psm {
namespace BitMatrixTracker {

// Forward declarations for internal helpers implemented in other translation units
bool computeTileValidityAndNormalized(const cv::Mat &gray,
                                      int tileSizePx,
                                      float peakThreshold,
                                      int tileHistogramBins,
                                      float minContrast,
                                      bool validateSubtiles,
                                      cv::Mat1b &outTileMask,
                                      cv::Mat &outNormalized);

bool buildClustersFromTileMask(const cv::Mat1b &tileMask,
                               int tileSizePx,
                               int minSideLengthTiles,
                               int minValidTilesCount,
                               float minValidAreaRatio,
                               float maxBoundsAspectRatio,
                               std::vector<Cluster> &outClusters);

bool detectCornersPerCluster(const cv::Mat& normalizedU8,
                             const Config& cfg,
                             const Cluster& cluster,
                             const CornerNetWeights& weights,
                             Detections& outRaw,
                             int* rawCountOut);

bool groupSplitAndCollapse(const Detections& raw,
                           const Config& cfg,
                           Detections& outDiag1,
                           Detections& outDiag2,
                           int* keptLoose,
                           int* keptStrict);

                           
bool estimateWithRansac(const cv::Mat &gray,
    const Config &cfg,
    const Target &target,
    const Detections &diag1,
    const Detections &diag2,
    const cv::Matx33d &cameraIntrinsics,
    float sizeFracMin,
    float sizeFracMax,
    cv::Matx33d &outH,
    Pose &outPose,
    bool &outFlipDiags,
    bool &outRot180,
    int &outInliers,
    int &outIterations,
    bool debug);


struct Estimator::Impl {
    explicit Impl(const Config &config)
        : m_config(config)
    {
        bool loaded = loadCornerNetWeightsFromFile(config.cornerNetWeightsPath, m_weights);
        if (!loaded) {
            std::cerr << "Estimator: failed to load CornerNet weights: " << config.cornerNetWeightsPath << std::endl;
        }
    }

    Config m_config;

    // Reusable buffers
    cv::Mat m_normalized;   // normalized copy after tile stretching
    cv::Mat1b m_tileMask;   // valid tiles mask

    // CornerNet weights (must be loaded by caller before detection)
    CornerNetWeights m_weights;
};

Estimator::Estimator(const Config &config)
    : m_impl(std::make_unique<Impl>(config))
{
}

Estimator::~Estimator() = default;

bool Estimator::computeTileClusters(const cv::Mat &gray,
                                    std::vector<Cluster> &outClusters,
                                    Diagnostics *diagnostics) const
{
    try {
        outClusters.clear();
        if (gray.empty() || gray.channels() != 1) {
            std::cerr << "computeTileClusters: invalid input image" << std::endl;
            return false;
        }

        // 1) Build tile validity + normalized image
        if (!computeTileValidityAndNormalized(gray,
                                              m_impl->m_config.tileSizePx,
                                              m_impl->m_config.peakThreshold,
                                              m_impl->m_config.tileHistogramBins,
                                              m_impl->m_config.minContrast,
                                              m_impl->m_config.validateSubtiles,
                                              m_impl->m_tileMask,
                                              m_impl->m_normalized))
            return false;

        if (diagnostics) {
            diagnostics->validTileCount = cv::countNonZero(m_impl->m_tileMask);
            std::cout << "validTileCount: " << diagnostics->validTileCount << std::endl;

            cv::Mat plot = cv::Mat::zeros(gray.rows, gray.cols, CV_8UC3);
            cv::cvtColor(m_impl->m_normalized, plot, cv::COLOR_GRAY2BGR);
            cv::Mat tileMask;
            m_impl->m_tileMask.convertTo(tileMask, CV_8UC1, -255, 255);
            cv::resize(tileMask, tileMask, cv::Size(plot.cols, plot.rows), 0, 0, cv::INTER_NEAREST);
            cv::Mat tileMaskRed = cv::Mat::zeros(plot.rows, plot.cols, CV_8UC3);
            cv::mixChannels(tileMask, tileMaskRed, {0, 2});
            cv::addWeighted(plot, 0.8, tileMaskRed, 0.2, 0, plot);
            cv::imwrite("tileMaskPlot.jpg", plot);
        }

        // 2) 8-connected clustering in tile space
        if (!buildClustersFromTileMask(m_impl->m_tileMask,
                                       m_impl->m_config.tileSizePx,
                                       m_impl->m_config.minClusterSideLengthTiles,
                                       m_impl->m_config.minClusterValidTilesCount,
                                       m_impl->m_config.minClusterValidAreaRatio,
                                       m_impl->m_config.maxClusterBoundsAspectRatio,
                                       outClusters)) {
            return false;
        }

        // Clamp pixel bounds to image size
        for (auto &c : outClusters) {
            c.pixelBounds.width = std::min(c.pixelBounds.width, std::max(0, gray.cols - c.pixelBounds.x));
            c.pixelBounds.height = std::min(c.pixelBounds.height, std::max(0, gray.rows - c.pixelBounds.y));
            //std::cout << "cluster pixel bounds (clamped): " << c.pixelBounds << std::endl;
        }

        return true;
    } catch (const std::exception &e) {
        std::cerr << "computeTileClusters exception: " << e.what() << std::endl;
        return false;
    }
}

bool Estimator::detectCornersInCluster(const cv::Mat &gray,
                                       const Cluster &cluster,
                                       Detections &outDiag1,
                                       Detections &outDiag2,
                                       Diagnostics *diagnostics) const
{
    try {
        outDiag1.points.clear();
        outDiag1.anglesDeg.clear();
        outDiag2.points.clear();
        outDiag2.anglesDeg.clear();
        if (gray.empty()) {
            std::cerr << "detectCornersInCluster: empty image" << std::endl;
            return false;
        }
        if (!m_impl->m_weights.isValid()) {
            std::cerr << "detectCornersInCluster: CornerNet weights not loaded" << std::endl;
            return false;
        }
        
        // Raw detections before angle grouping/collapse
        Detections raw;
        int rawCount = 0;

        if (diagnostics) {
            cv::Mat plot;
            cv::cvtColor(m_impl->m_normalized, plot, cv::COLOR_GRAY2BGR);
            // INSERT_YOUR_CODE
            // Draw the pixel bounds of the cluster on the plot as a rectangle (blue)
            cv::rectangle(plot, cluster.pixelBounds, cv::Scalar(255, 0, 200), 2);
            cv::imwrite("clusterPlot.jpg", plot);
        }

        // Use normalized per-tile image within cluster bounds; Detect.cpp handles tile loop.
        if (!detectCornersPerCluster(m_impl->m_normalized, m_impl->m_config, cluster,
                                     m_impl->m_weights, raw, &rawCount)) {
            std::cerr << "detectCornersInCluster: detectCornersPerCluster failed" << std::endl;
            return false;
        }

        if (diagnostics) {
            std::cout << "raw corners count: " << rawCount << std::endl;
            diagnostics->rawCorners = rawCount;

            cv::Mat plot;
            cv::cvtColor(m_impl->m_normalized, plot, cv::COLOR_GRAY2BGR);
            const float lineLength = 10;
            for (size_t i = 0; i < raw.points.size(); ++i) {
                cv::circle(plot, raw.points[i], 2, cv::Scalar(0, 255, 0), -1);
                cv::Vec2d offset = directionVec(raw.anglesDeg[i]);
                cv::Point2f lineTo = cv::Point2f(
                    raw.points[i].x + offset[0] * lineLength,
                    raw.points[i].y + offset[1] * lineLength
                );
                cv::line(plot, raw.points[i], lineTo, cv::Scalar(0, 255, 0), 1);
            }
            cv::imwrite("rawCornersPlot.jpg", plot);

            std::cout << "[BitMatrixTracker] Raw detections in cluster: " << rawCount << std::endl;
            //for (size_t i = 0; i < raw.anglesDeg.size(); ++i) {
            //    std::cout << "angle " << i << ": " << raw.anglesDeg[i] << std::endl;
            //}
        }


        int nLoose = 0, nStrict = 0;
        if (!groupSplitAndCollapse(raw, m_impl->m_config, outDiag1, outDiag2, &nLoose, &nStrict)) {
            std::cerr << "detectCornersInCluster: groupSplitAndCollapse failed\n";
            return false;
        }

        /*
        for (size_t i = 0; i < outDiag2.points.size(); ++i) {
            outDiag2.anglesDeg[i] = std::fmod(outDiag2.anglesDeg[i] + 90.0f, 180.0f);
        }
        */

        if (diagnostics) {
            diagnostics->keptCornersLoose  = nLoose;
            diagnostics->keptCornersStrict = nStrict;

            cv::Mat plot;
            cv::cvtColor(m_impl->m_normalized, plot, cv::COLOR_GRAY2BGR);
            const float lineLength = 10;
            for (size_t i = 0; i < outDiag1.points.size(); ++i) {
                cv::circle(plot, outDiag1.points[i], 2, cv::Scalar(255, 255, 0), -1);
                cv::Vec2d offset = directionVec(outDiag1.anglesDeg[i]);
                cv::Point2f lineTo = cv::Point2f(
                    outDiag1.points[i].x + offset[0] * lineLength,
                    outDiag1.points[i].y + offset[1] * lineLength);
                cv::line(plot, outDiag1.points[i], lineTo, cv::Scalar(255, 255, 0), 1);
            }
            for (size_t i = 0; i < outDiag2.points.size(); ++i) {
                cv::circle(plot, outDiag2.points[i], 2, cv::Scalar(255, 0, 255), -1);
                cv::Vec2d offset = directionVec(outDiag2.anglesDeg[i]);
                cv::Point2f lineTo = cv::Point2f(
                    outDiag2.points[i].x + offset[0] * lineLength,
                    outDiag2.points[i].y + offset[1] * lineLength);
                cv::line(plot, outDiag2.points[i], lineTo, cv::Scalar(255, 0, 255), 1);
            }
            cv::imwrite("groupedCornersPlot.jpg", plot);
            std::cout << "keptCornersLoose: " << nLoose << std::endl;
            std::cout << "keptCornersStrict: " << nStrict << std::endl;
            std::cout << "Corners after grouping: d1=" << outDiag1.points.size()
                      << ", d2=" << outDiag2.points.size() << std::endl;
        }

        return true;
    } catch (const std::exception &e) {
        std::cerr << "detectCornersInCluster exception: " << e.what() << std::endl;
        return false;
    }
}

bool Estimator::estimatePose(const cv::Mat &gray,
                             const cv::Matx33d &cameraIntrinsics,
                             const Target &target,
                             const Detections &diag1,
                             const Detections &diag2,
                             float sizeFracMin,
                             float sizeFracMax,
                             Pose &outPose,
                             cv::Matx33d &outH,
                             Diagnostics *diagnostics) const
{
    try {
        int inliers = 0;
        int iterations = 0;
        bool flipDiags = false;
        bool rot180 = false;
        
        bool foundPose = estimateWithRansac(
            gray, m_impl->m_config, target, diag1, diag2, cameraIntrinsics, sizeFracMin, sizeFracMax,
            outH, outPose, flipDiags, rot180, inliers, iterations,
            diagnostics != nullptr
        );

        if (!foundPose) {
            std::cout << "estimatePose: no homography found" << std::endl;
            return false;
        }

        if (diagnostics) {
            diagnostics->inliersBest = inliers;
            diagnostics->ransacIterations = iterations;
        }

        if (diagnostics) {
            std::cout << "flipDiags: " << (flipDiags ? "true" : "false") << std::endl;
            std::cout << "rot180: " << (rot180 ? "true" : "false") << std::endl;
        }

        return true;

    } catch (const std::exception &e) {
        std::cerr << "estimatePose exception: " << e.what() << std::endl;
        return false;
    }
}

bool Estimator::estimatePose(const cv::Mat &gray,
                             const cv::Matx33d &K,
                             const Target &target,
                             Pose &outPose,
                             cv::Matx33d &outH,
                             Diagnostics *diagnostics) const
{
    std::vector<Cluster> clusters;
    if (!computeTileClusters(gray, clusters, diagnostics))
        return false;

    if (clusters.empty()) {
        std::cout << "No clusters found in image" << std::endl;
        return false;
    }

    // Pick biggest cluster by tile area
    size_t bestIndex = 0;
    int bestArea = -1;
    for (size_t i = 0; i < clusters.size(); ++i) {
        int area = clusters[i].tileBounds.area();
        if (area > bestArea) { bestArea = area; bestIndex = static_cast<int>(i); }
    }

    if (diagnostics) {
        std::cout << "clusters: " << clusters.size() << std::endl;
        std::cout << "best cluster index " << bestIndex << ", tile bounds " << clusters[bestIndex].tileBounds << ", pixel bounds " << clusters[bestIndex].pixelBounds << std::endl;
        diagnostics->clusterCount = static_cast<int>(clusters.size());
        diagnostics->bestClusterIndex = static_cast<int>(bestIndex);
    }

    Detections d1, d2;
    if (!detectCornersInCluster(gray, clusters[bestIndex], d1, d2, diagnostics)) {
        std::cout << "No corners detected in cluster " << bestIndex << std::endl;
        return false;
    }
    
    int clusterSize =  std::max(clusters[bestIndex].pixelBounds.width, clusters[bestIndex].pixelBounds.height);
    int minDim = std::min(gray.cols, gray.rows);
    float clusterFrac = static_cast<float>(clusterSize) / static_cast<float>(minDim);
    int minPixelSize = target.bitmatrix.size[0] * 2; // Very small in screen space
    float verySmallFrac = static_cast<float>(minPixelSize) / static_cast<float>(minDim);
    float sizeFracMin = std::max(clusterFrac * 0.5f, verySmallFrac);
    float sizeFracMax = std::min(clusterFrac * 1.5f, 0.95f);

    bool foundPose = estimatePose(
        gray, K, target, d1, d2, sizeFracMin, sizeFracMax,
        outPose, outH, diagnostics);
    

    // TEST!!
    // Try moving photo points into marker space and plot if it looks good. Maybe we could do more of the pose finding in marker space to reduce heavy math / ransac?
    if (foundPose && diagnostics) {
        cv::Matx33d H_inv = cv::Matx33d::zeros();
        if (!cv::invert(outH, H_inv)) {
            std::cerr << "Failed to invert homography" << std::endl;
            return false;
        }

        std::vector<cv::Point2f> unprojectedPoints1, unprojectedPoints2;
        cv::perspectiveTransform(d1.points, unprojectedPoints1, H_inv);
        cv::perspectiveTransform(d2.points, unprojectedPoints2, H_inv);
        const int plotScale = 40;
        const int plotSize = target.bitmatrix.size[0] * 2 * plotScale;
        const int plotOffset = plotSize / 2 - target.bitmatrix.size[0] / 2 * plotScale; // center target in plot, and more space around it
        cv::Mat plot = cv::Mat::zeros(plotSize, plotSize, CV_8UC3);
        auto plotPoint = [plotSize, plotOffset, plotScale] (const cv::Point2f &point) -> cv::Point {
            return cv::Point(
                point.x * plotScale + plotOffset,
                point.y * plotScale + plotOffset
            );
        };
        // plot target rect
        const auto& targetCorners = calcTargetSpaceCorners(target.bitmatrix.size[0]);
        std::vector<cv::Point2i> targetCornersPlot(4);
        for (int i = 0; i < targetCorners.size(); i++) {
            targetCornersPlot[i] = plotPoint(targetCorners[i]);
        }
        std::vector<std::vector<cv::Point2i>> contours = { targetCornersPlot };
        cv::drawContours(plot, contours, 0, cv::Scalar(0, 255, 0), 1);

        std::vector<cv::Point2i> targetPoints1(target.diag1.size());
        for (int i = 0; i < target.diag1.size(); i++) {
            targetPoints1[i] = plotPoint(target.diag1[i]);
        }
        std::vector<cv::Point2i> targetPoints2(target.diag2.size());
        for (int i = 0; i < target.diag2.size(); i++) {
            targetPoints2[i] = plotPoint(target.diag2[i]);
        }
        for (const auto &point : targetPoints1) {
            cv::circle(plot, point, 4, cv::Scalar(255, 255, 0), 0.7f);
        }
        for (const auto &point : targetPoints2) {
            cv::circle(plot, point, 4, cv::Scalar(255, 0, 255), 0.7f);
        }

        for (const auto &point : unprojectedPoints1) {
            auto p = plotPoint(point);
            //std::cout << "unprojected point 1: " << point << ", plot point: " << p << std::endl;
            if (p.x < 0 || p.x >= plotSize || p.y < 0 || p.y >= plotSize)
                continue;
            cv::circle(plot, p, 2, cv::Scalar(255, 255, 0), -1);
        }
        for (const auto &point : unprojectedPoints2) {
            auto p = plotPoint(point);
            //std::cout << "unprojected point 2: " << point << ", plot point: " << p << std::endl;
            if (p.x < 0 || p.x >= plotSize || p.y < 0 || p.y >= plotSize)
                continue;
            cv::circle(plot, p, 2, cv::Scalar(255, 0, 255), -1);
        }
        cv::imwrite("unprojectedCornersPlot.jpg", plot);
    }


    return foundPose;
}

} // namespace BitMatrixTracker
} // namespace psm
