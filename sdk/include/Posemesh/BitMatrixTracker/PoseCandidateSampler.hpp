#pragma once

#include "Posemesh/BitMatrixTracker/Config.hpp"
#include "Posemesh/BitMatrixTracker/Types.hpp"
#include "Posemesh/BitMatrixTracker/Geometry.hpp"

#include <opencv2/core.hpp>
#include <random>
#include <vector>

namespace psm {
namespace BitMatrixTracker {

class PoseCandidateSampler {
public:

    PoseCandidateSampler(const Config &cfg,
                         const Target &target,
                         const Detections &diag1,
                         const Detections &diag2,
                         const cv::Matx33d &cameraIntrinsics,
                         const cv::Size &imageSize,
                         float sizeFracMin = 0.05f,
                         float sizeFracMax = 1.0f);

    void seedRandom(unsigned int seed);

    void reset();

    void report(bool good);

    bool generate(cv::Matx33d &outHomography, bool &outFlipDiags, bool &outRot180);

private:
    const Config &m_cfg;
    const Target &m_target;
    const Detections &m_diag1;
    const Detections &m_diag2;
    float m_sizeFracMin;
    float m_sizeFracMax;
    const cv::Matx33d &m_cameraIntrinsics;
    cv::Size m_imgSize;

    std::mt19937 m_rng;
    std::vector<float> m_bins;
    int m_lastBin {-1};
};

} // namespace BitMatrixTracker
} // namespace psm
