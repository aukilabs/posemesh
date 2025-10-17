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
                               std::vector<Cluster> &outClusters,
                               int minSideLengthTiles,
                               int minValidTilesCount);

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
    cv::Matx33d &outH,
    bool &outFlipDiags,
    int &outInliers,
    int &outIterations);


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
            c.pixelBounds.width = std::min(c.pixelBounds.width, std::max(0, gray.cols - c.pixelBounds.x));
            c.pixelBounds.height = std::min(c.pixelBounds.height, std::max(0, gray.rows - c.pixelBounds.y));
            std::cout << "cluster pixel bounds (clamped): " << c.pixelBounds << std::endl;
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
        if (!m_impl->m_weights.isValid()) {
            std::cerr << "detectCornersInCluster: CornerNet weights not loaded" << std::endl;
            return false;
        }
        
        // Raw detections before angle grouping/collapse
        Detections raw;
        int rawCount = 0;

        // Use normalized per-tile image within cluster bounds; Detect.cpp handles tile loop.
        if (!detectCornersPerCluster(m_impl->m_normalized, m_impl->m_config, cluster,
                                     m_impl->m_weights, raw, &rawCount)) {
            std::cerr << "detectCornersInCluster: detectCornersPerCluster failed" << std::endl;
            return false;
        }

        if (diag) {
            diag->rawCorners = rawCount;

            cv::Mat plot;
            cv::cvtColor(m_impl->m_normalized, plot, cv::COLOR_GRAY2BGR);
            for (size_t i = 0; i < raw.points.size(); ++i) {
                cv::circle(plot, raw.points[i], 2, cv::Scalar(0, 255, 0), -1);
            }
            cv::imwrite("rawCornersPlot.jpg", plot);
        }
        std::cout << "[BitMatrixTracker] Raw detections in cluster: " << rawCount << std::endl;
        //for (size_t i = 0; i < raw.anglesDeg.size(); ++i) {
        //    std::cout << "angle " << i << ": " << raw.anglesDeg[i] << std::endl;
        //}


        int nLoose = 0, nStrict = 0;
        if (!groupSplitAndCollapse(raw, m_impl->m_config, outDiag1, outDiag2, &nLoose, &nStrict)) {
            std::cerr << "detectCornersInCluster: groupSplitAndCollapse failed\n";
            return false;
        }
        if (diag) {
            diag->keptCornersLoose  = nLoose;
            diag->keptCornersStrict = nStrict;

            cv::Mat plot;
            cv::cvtColor(m_impl->m_normalized, plot, cv::COLOR_GRAY2BGR);
            for (size_t i = 0; i < outDiag1.points.size(); ++i) {
                cv::circle(plot, outDiag1.points[i], 2, cv::Scalar(255, 255, 0), -1);
            }
            for (size_t i = 0; i < outDiag2.points.size(); ++i) {
                cv::circle(plot, outDiag2.points[i], 2, cv::Scalar(255, 0, 255), -1);
            }
            cv::imwrite("groupedCornersPlot.jpg", plot);
        }

        std::cout << "keptCornersLoose: " << nLoose << std::endl;
        std::cout << "keptCornersStrict: " << nStrict << std::endl;
        
        std::cout << "[BitMatrixTracker] After grouping: d1=" << outDiag1.points.size()
                << ", d2=" << outDiag2.points.size() << std::endl;

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
                             Pose &outPose,
                             cv::Matx33d &outH,
                             Diagnostics *diag) const
{
    try {
        int inliers = 0;
        int iterations = 0;
        bool outFlipDiags = false;
        bool foundHomography = estimateWithRansac(gray, m_impl->m_config, target, diag1, diag2, cameraIntrinsics, outH, outFlipDiags, inliers, iterations);

        if (!foundHomography) {
            std::cout << "estimatePose: no homography found" << std::endl;
            return false;
        }

        std::cout << "FOUND Homography:" << std::endl << outH << std::endl;
        std::cout << "Inliers = " << inliers << std::endl;
        std::cout << "Iterations = " << iterations << std::endl;

        if (diag) {
            diag->inliersBest = inliers;
            diag->ransacIterations = iterations;
        }

        std::vector<cv::Point2f> markerCorners = {
            {0.0, 0.0},
            {21.0, 0.0},
            {21.0, 21.0},
            {0.0, 21.0}
        };

        std::vector<cv::Point2i> projectedCornersInt;
        projectWithH(markerCorners, outH, projectedCornersInt);
        std::vector<cv::Point2d> projectedCorners;
        for (const auto &p : projectedCornersInt) {
            projectedCorners.push_back(cv::Point2d(p.x, p.y));
        }

        const double halfSide = target.sideLengthMeters / 2.0;
        std::vector<cv::Point3d> objectCorners = {
            {-halfSide, halfSide, 0.0},
            {halfSide, halfSide, 0.0},
            {halfSide, -halfSide, 0.0},
            {-halfSide, -halfSide, 0.0}
        };

        std::cout << "num projected corners = " << projectedCorners.size() << std::endl;
        std::cout << "num object corners = " << objectCorners.size() << std::endl;
        std::cout << "projected corners = " << projectedCorners << std::endl;
        std::cout << "object corners = " << objectCorners << std::endl;

        cv::Mat rvec, tvec;
        bool gotPose = cv::solvePnP(objectCorners, projectedCorners, cameraIntrinsics, cv::noArray(), rvec, tvec, false, cv::SOLVEPNP_ITERATIVE);
        if (!gotPose) {
            std::cout << "estimatePose: solvePnP found no pose" << std::endl;
            return false;
        }

        // TODO: final refinement with ALL inliers (if needed, but maybe doing it inside ransac loop is enough)
        outPose.rvec = rvec;
        outPose.tvec = tvec;

        std::cout << "solvePnP: rvec = " << rvec.t() << std::endl;
        std::cout << "solvePnP: tvec = " << tvec.t() << std::endl;

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
    if (!detectCornersInCluster(gray, clusters[bestIndex], d1, d2, diag)) {
        std::cout << "detectCornersInCluster returned false" << std::endl;
        return false;
    }
    
    bool foundPose = estimatePose(gray, K, target, d1, d2, outPose, outH, diag);
    std::cout << "Found pose? = " << foundPose << std::endl;
    return foundPose;
}

} // namespace BitMatrixTracker
} // namespace psm
