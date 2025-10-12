#include <iostream>
#include <memory>
#include <algorithm>
#include <numeric>
#include <opencv2/imgcodecs.hpp>
#include <opencv2/imgproc.hpp>
#include "Posemesh/BitMatrixTracker/Estimator.hpp"

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
                               std::vector<Cluster> &outClusters,
                               int minSideLengthTiles,
                               int minValidTilesCount);

struct Estimator::Impl {
    explicit Impl(const Config &config)
        : m_config(config)
    {
    }

    Config m_config;

    // Reusable buffers
    cv::Mat m_normalized;   // normalized copy after tile stretching
    cv::Mat1b m_tileMask;   // valid tiles mask
};

Estimator::Estimator(const Config &config)
    : m_impl(std::make_unique<Impl>(config))
{
}

Estimator::~Estimator() = default;

bool Estimator::computeTileClusters(const cv::Mat &gray,
                                    std::vector<Cluster> &outClusters,
                                    Diagnostics *diag) const
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

        if (diag) {
            diag->validTileCount = cv::countNonZero(m_impl->m_tileMask);
            std::cout << "validTileCount: " << diag->validTileCount << std::endl;

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
                                       outClusters,
                                       m_impl->m_config.minClusterSideLengthTiles,
                                       m_impl->m_config.minClusterValidTilesCount))
            return false;

        // Clamp pixel bounds to image size
        std::cout << "Found " << outClusters.size() << " clusters" << std::endl;
        for (auto &c : outClusters) {
            std::cout << "cluster pixel bounds: " << c.pixelBounds << std::endl;
            c.pixelBounds.width = std::min(c.pixelBounds.width, gray.cols - c.pixelBounds.x);
            c.pixelBounds.height = std::min(c.pixelBounds.height, gray.rows - c.pixelBounds.y);
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
                                       Diagnostics *diag) const
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

        // TODO(next): use m_impl->m_normalized within cluster.pixelBounds
        // iterate per valid tile in cluster.tileMask and slide 5x5 -> CornerNet
        // then global angle grouping across the cluster

        (void)cluster; (void)diag;
        return true; // placeholder
    } catch (const std::exception &e) {
        std::cerr << "detectCornersInCluster exception: " << e.what() << std::endl;
        return false;
    }
}

bool Estimator::estimatePose(const cv::Mat &gray,
                             const cv::Matx33d &K,
                             const Target &target,
                             const Detections &diag1,
                             const Detections &diag2,
                             Pose &outPose,
                             cv::Matx33d &outH,
                             Diagnostics *diag) const
{
    try {
        (void)gray;
        (void)K;
        (void)target;
        (void)diag1;
        (void)diag2;
        (void)diag;

        // TODO(next): collapse -> NearbyMask -> RANSAC -> PnP

        outH = cv::Matx33d::eye();
        outPose.rvec = cv::Vec3d(0, 0, 0);
        outPose.tvec = cv::Vec3d(0, 0, 0);
        return true; // placeholder
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
                             Diagnostics *diag) const
{
    std::vector<Cluster> clusters;
    if (!computeTileClusters(gray, clusters, diag))
        return false;

    if (clusters.empty()) {
        std::cerr << "estimatePose: no clusters" << std::endl;
        return false;
    }
    // Pick biggest cluster by tile area
    size_t bestIndex = 0;
    int bestArea = -1;
    for (size_t i = 0; i < clusters.size(); ++i) {
        int area = clusters[i].tileBounds.area();
        if (area > bestArea) { bestArea = area; bestIndex = static_cast<int>(i); }
    }
    if (diag) {
        diag->clusterCount = static_cast<int>(clusters.size());
        diag->bestClusterIndex = static_cast<int>(bestIndex);
    }
    Detections d1, d2;
    if (!detectCornersInCluster(gray, clusters[bestIndex], d1, d2, diag))
        return false;
    return estimatePose(gray, K, target, d1, d2, outPose, outH, diag);
}

} // namespace BitMatrixTracker
} // namespace psm
