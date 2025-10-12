#pragma once

#include "Config.hpp"
#include "Types.hpp"

namespace psm {
namespace BitMatrixTracker {

class Estimator {
public:
    explicit Estimator(const Config &config = defaultConfig());
    ~Estimator();

    // Stage 1: compute all clusters; caller may choose biggest.
    bool computeTileClusters(const cv::Mat &gray,
                             std::vector<Cluster> &outClusters,
                             Diagnostics *diag = nullptr) const;

    // Stage 2: per-cluster sliding window detection (valid tiles only).
    bool detectCornersInCluster(const cv::Mat &gray,
                                const Cluster &cluster,
                                Detections &outDiag1,
                                Detections &outDiag2,
                                Diagnostics *diag = nullptr) const;

    // Stage 3: estimate pose (uses NearbyMask with one-to-one matching).
    bool estimatePose(const cv::Mat &gray,
                      const cv::Matx33d &K,
                      const Target &target,
                      const Detections &diag1,
                      const Detections &diag2,
                      Pose &outPose,
                      cv::Matx33d &outH,
                      Diagnostics *diag = nullptr) const;

    // Convenience: run 1–3 with the biggest cluster.
    bool estimatePose(const cv::Mat &gray,
                      const cv::Matx33d &K,
                      const Target &target,
                      Pose &outPose,
                      cv::Matx33d &outH,
                      Diagnostics *diag = nullptr) const;

private:
    struct Impl;
    std::unique_ptr<Impl> m_impl;
};

} // namespace BitMatrixTracker
} // namespace psm