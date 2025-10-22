#include <random>
#include <vector>
#include <iostream>
#include <limits>
#include <chrono>
#include <opencv2/core.hpp>
#include <opencv2/calib3d.hpp>
#include <opencv2/imgproc.hpp>
#include <opencv2/imgcodecs.hpp>

#include "Posemesh/BitMatrixTracker/Config.hpp"
#include "Posemesh/BitMatrixTracker/Types.hpp"
#include "Posemesh/BitMatrixTracker/Geometry.hpp"
#include "Posemesh/BitMatrixTracker/PoseCandidateSampler.hpp"

namespace psm {
namespace BitMatrixTracker {

// NearbyMask helpers (internal linkage provided by NearbyMask.cpp)
bool buildLabelMap(const cv::Size &imgSize,
                   const std::vector<cv::Point2f> &centers,
                   float radiusPx,
                   cv::Mat1s &outLabel);
int countInliersOneToOne(const std::vector<cv::Point2f> &proj,
                         const cv::Mat1s &label);

// --- RANSAC driver ----------------------------------------------------------
struct RansacResult {
    cv::Matx33d H {cv::Matx33d::eye()};
    bool flipDiags {false};
    int inliers {0};
    int iterations {0};
};

void plotNearbyMask(const cv::Mat &gray, const cv::Mat1s &label, const std::string &filename) {
    cv::Mat plot = cv::Mat::zeros(gray.rows, gray.cols, CV_8UC3);
    cv::cvtColor(gray, plot, cv::COLOR_GRAY2BGR);
    for (int y = 0; y < label.rows; ++y) {
        for (int x = 0; x < label.cols; ++x) {
            int labelVal = label(y, x);
            if (labelVal >= 0) {
                const cv::Scalar color = cv::Scalar(255 - (20 * labelVal) % 200, (33 * labelVal) % 255, 30);
                cv::circle(plot, cv::Point(x, y), 1, color, -1);
            }
        }
    }
    cv::imwrite(filename, plot);
}

static bool ransacHomography(const Config &cfg,
                             const Target &target,
                             const Detections &diag1,
                             const Detections &diag2,
                             const cv::Matx33d &cameraIntrinsics,
                             const cv::Size &imageSize,
                             const cv::Mat1s &label1,
                             const cv::Mat1s &label2,
                             RansacResult &out)
{
    try {
        PoseCandidateSampler sampler(cfg, target, diag1, diag2, cameraIntrinsics, imageSize);
        std::vector<cv::Point2f> proj1, proj2;
        const int maxIters = std::max(1, cfg.ransacMaxIters);
        const int targetMax = static_cast<int>(target.diag1.size() + target.diag2.size());
        const int earlyStopAt = (cfg.earlyStopPercent > 0) ? (targetMax * cfg.earlyStopPercent / 100) : std::numeric_limits<int>::max();

        out.inliers = 0;
        out.iterations = 0;

        auto startTime = std::chrono::high_resolution_clock::now();

        for (int it = 0; it < maxIters; ++it) {
            out.iterations = it + 1;

            cv::Matx33d H;
            bool flippedDiags = false;
            if (!sampler.generate(H, flippedDiags))
                continue;

            // Project families
            projectWithH(target.diag1, H, proj1);
            projectWithH(target.diag2, H, proj2);

            // Score via NearbyMask one-to-one
            auto& labelsForProj1 = label1;// flippedDiags ? label2 : label1;
            auto& labelsForProj2 = label2;// flippedDiags ? label1 : label2;
            const int s1 = countInliersOneToOne(proj1, labelsForProj1);
            const int s2 = countInliersOneToOne(proj2, labelsForProj2);
            const int score = s1 + s2;

            const bool improved = (score > out.inliers);
            if (improved) {
                out.inliers = score;
                out.H = H;
                out.flipDiags = flippedDiags;
                std::cout << "Ransac improved: score = " << score << " (diag1: " << s1 << ", diag2: " << s2 << ")" << std::endl;
            }
            //sampler.report(improved);

            if (out.inliers >= earlyStopAt) {
                std::cout << "Ransac stopping early: reached " << out.inliers << " inliers" << std::endl;
                break;
            }
        }

        auto endTime = std::chrono::high_resolution_clock::now();
        auto duration = std::chrono::duration_cast<std::chrono::microseconds>(endTime - startTime).count();
        std::cout << "Ransac took " << (duration / 1000) << " ms" << std::endl;

        double timePerIter = duration / out.iterations;
        std::cout << "Time per iteration: " << (timePerIter) << " microseconds" << std::endl;

        return true;
    } catch (const std::exception &e) {
        std::cerr << "ransacHomography exception: " << e.what() << std::endl;
        return false;
    }
}

bool estimateWithRansac(const cv::Mat &gray,
                        const Config &cfg,
                        const Target &target,
                        const Detections &diag1,
                        const Detections &diag2,
                        const cv::Matx33d &cameraIntrinsics,
                        cv::Matx33d &outH,
                        bool &outFlipDiags,
                        int &outInliers,
                        int &outIterations)
{
    try {
        // Build NearbyMask label maps
        cv::Mat1s label1, label2;

        if (!buildLabelMap(gray.size(), diag1.points, cfg.inlierRadiusPx, label1)) {
            std::cout << "buildLabelMap diag1 failed" << std::endl;
            return false;
        }

        if (!buildLabelMap(gray.size(), diag2.points, cfg.inlierRadiusPx, label2)) {
            std::cout << "buildLabelMap diag2 failed" << std::endl;
            return false;
        }

        std::cout << "label1.size() = " << label1.size() << std::endl;
        std::cout << "label2.size() = " << label2.size() << std::endl;

        plotNearbyMask(gray, label1, "nearbyMask1.jpg");
        plotNearbyMask(gray, label2, "nearbyMask2.jpg");

        RansacResult result;
        if (!ransacHomography(cfg, target, diag1, diag2, cameraIntrinsics, gray.size(), label1, label2, result)) {
            std::cout << "ransacHomography failed" << std::endl;
            return false;
        }

        outH = result.H;
        outFlipDiags = result.flipDiags;
        outInliers = result.inliers;
        outIterations = result.iterations;
        std::cout << "estimateWithRansac succeed with " << outInliers
                  << " inliers after " << outIterations << " iterations"
                  << std::endl;

        return true;
    } catch (const std::exception &e) {
        std::cerr << "estimateWithRansac exception: " << e.what() << std::endl;
        return false;
    }
}

} // namespace BitMatrixTracker
} // namespace psm
