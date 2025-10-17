#include "Posemesh/BitMatrixTracker/PoseCandidateSampler.hpp"
#include "Posemesh/BitMatrixTracker/Config.hpp"
#include "Posemesh/BitMatrixTracker/Types.hpp"
#include "Posemesh/BitMatrixTracker/Geometry.hpp"

#include <iostream>
#include <random>

namespace psm {
namespace BitMatrixTracker {

PoseCandidateSampler::PoseCandidateSampler(const Config &cfg,
                                         const Target &target,
                                         const Detections &diag1,
                                         const Detections &diag2,
                                         const cv::Matx33d &cameraIntrinsics,
                                         const cv::Size &imageSize)
    : m_cfg(cfg)
    , m_target(target)
    , m_diag1(diag1)
    , m_diag2(diag2)
    , m_cameraIntrinsics(cameraIntrinsics)
    , m_imgSize(imageSize)
    , m_rng(1234567u) // caller can replace via setter later
{
}

void PoseCandidateSampler::reset()
{
}

void PoseCandidateSampler::report(bool good)
{
}

bool PoseCandidateSampler::generate(cv::Matx33d &outHomography, bool &outFlipDiags)
{
    if (m_diag1.points.empty() && m_diag2.points.empty())
        return false;
    const bool flipDiags = (m_diag2.points.empty()) || (!m_diag1.points.empty() && std::uniform_int_distribution<int>(0,1)(m_rng)==0);
    outFlipDiags = flipDiags;

    auto& points1 = flipDiags ? m_diag2.points : m_diag1.points;
    auto& points2 = flipDiags ? m_diag1.points : m_diag2.points;

    // ----------
    // Pick position based on one random point correspondence
    // ----------
    const int correspondenceDiag = std::uniform_int_distribution<int>(0,1)(m_rng);
    const auto& targetCorrespondencePoints = correspondenceDiag ? m_target.diag2 : m_target.diag1;
    const auto& detectedCorrespondencePoints = correspondenceDiag ? points2 : points1;

    int targetIndex = std::uniform_int_distribution<int>(0, targetCorrespondencePoints.size() - 1)(m_rng);
    int detectedIndex = std::uniform_int_distribution<int>(0, detectedCorrespondencePoints.size() - 1)(m_rng);

    const auto& targetPoint = targetCorrespondencePoints[targetIndex];
    const auto& detectedPoint = detectedCorrespondencePoints[detectedIndex];

    // ----------
    // Pick random up/right vector
    // ----------
    bool randomAngleDiag = std::uniform_int_distribution<int>(0,1)(m_rng)==0;
    const auto& detectedUpAngles = randomAngleDiag ? m_diag1.anglesDeg : m_diag2.anglesDeg;
    const auto& detectedRightAngles = randomAngleDiag ? m_diag2.anglesDeg : m_diag1.anglesDeg;

    int upAngleIndex = std::uniform_int_distribution<int>(0, detectedUpAngles.size() - 1)(m_rng);
    double detectedUpAngle = detectedUpAngles[upAngleIndex];

    // Try to pick right and up vectors that are reasonably close to orthogonal.
    int rightAngleIndex = 0;
    double detectedRightAngle = 0.0;
    for (int attempt = 0; attempt < 100; ++attempt) {
        rightAngleIndex = std::uniform_int_distribution<int>(0, detectedRightAngles.size() - 1)(m_rng);
        detectedRightAngle = detectedRightAngles[rightAngleIndex];

        // Check if the angle difference is close to 90 degrees, within a tolerance.
        double delta = std::fmod((detectedRightAngle - detectedUpAngle + 180.0), 360.0) - 180.0;
        double degreesOff = std::abs(std::abs(delta) - 90.0);
        double tolerance = 10.0;
        if (degreesOff <= tolerance) {
            break; // Close enough!
        }
    }

    // Some extra random jittering
    detectedUpAngle += std::uniform_real_distribution<double>(-2.0, 2.0)(m_rng);
    detectedRightAngle += std::uniform_real_distribution<double>(-2.0, 2.0)(m_rng);

    cv::Vec2d detectedUpVec = directionVec(detectedUpAngle);
    cv::Vec2d detectedRightVec = directionVec(detectedRightAngle);

    // Ensure consistent axis order for later math to work (not get flipped or mirrored etc)
    double signedAngle = signedAngle2D(detectedUpVec, detectedRightVec);
    if (signedAngle < 0.0) {
        detectedUpVec *= -1.0;
    }

    double sizeFrac = std::uniform_real_distribution<double>(m_cfg.sizeFracMin, m_cfg.sizeFracMax)(m_rng);

    bool success = homographyFromPointAndDirs(
        targetPoint,
        detectedPoint,
        detectedRightVec,
        detectedUpVec,
        m_imgSize.width,
        m_imgSize.height,
        m_target.bitmatrix.size[0],
        m_cameraIntrinsics,
        sizeFrac,
        outHomography,
        m_rng,
        true,
        1.0e-9
    );

    if (!success) {
        //std::cout << "Homography candidate not valid" << std::endl;
        return false;
    }

    //std::cout << "Homography candidate valid! H = " << outHomography << " flipDiags = " << outFlipDiags << std::endl;

    return true;
}

} // namespace BitMatrixTracker
} // namespace psm