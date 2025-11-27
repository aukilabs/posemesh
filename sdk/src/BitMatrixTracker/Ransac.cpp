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
#include "Posemesh/BitMatrixTracker/FastHomography.hpp"

namespace psm {
namespace BitMatrixTracker {

// NearbyMask helpers (internal linkage provided by NearbyMask.cpp)
bool buildNearbyMask(const cv::Size &imgSize,
                     const std::vector<cv::Point2f> &centers,
                     float radiusPx,
                     std::vector<int16_t> &outNearbyMask);

int countInliers(const std::vector<cv::Point2i> &proj,
                 const std::vector<int16_t> &nearbyMask,
                 const int W, const int H);

int countInliersOneToOne(const std::vector<cv::Point2f> &proj,
                         const std::vector<int16_t> &nearbyMask,
                         const int W, const int H,
                         std::vector<int> &outProjInlierIndices,
                         std::vector<int16_t> &outNearbyMaskInliers);

// --- RANSAC driver ----------------------------------------------------------
struct RansacResult {
    cv::Matx33d H {cv::Matx33d::eye()};
    cv::Vec3d rvec {0, 0, 0};
    cv::Vec3d tvec {0, 0, 0};
    bool flipDiags {false};
    bool rot180 {false};
    int inliers {0};
    int iterations {0};
};

void plotNearbyMask(const cv::Mat &gray, const std::vector<int16_t> &label, const std::string &filename) {
    if (label.size() != gray.rows * gray.cols) {
        std::cerr << "plotNearbyMask: label size mismatch (label.size(): " << label.size() << ", gray.rows: " << gray.rows << ", gray.cols: " << gray.cols << ")" << std::endl;
        return;
    }

    cv::Mat plot = cv::Mat::zeros(gray.rows, gray.cols, CV_8UC3);
    cv::cvtColor(gray, plot, cv::COLOR_GRAY2BGR);
    for (int y = 0; y < gray.rows; ++y) {
        for (int x = 0; x < gray.cols; ++x) {
            int labelVal = label[y * gray.cols + x];
            if (labelVal >= 0) {
                const cv::Scalar color = cv::Scalar(255 - (20 * labelVal) % 200, (33 * labelVal) % 255, 30);
                cv::circle(plot, cv::Point(x, y), 1, color, -1);
            }
        }
    }
    cv::imwrite(filename, plot);
}

static std::vector<cv::Point2f> unprojectedPoints1, unprojectedPoints2;

static bool ransacHomography(const Config &cfg,
                             const Target &target,
                             const Detections &diag1,
                             const Detections &diag2,
                             const cv::Matx33d &cameraIntrinsics,
                             const cv::Size &imageSize,
                             const std::vector<int16_t> &label1,
                             const std::vector<int16_t> &label2,
                             int maskW, int maskH,
                             float sizeFracMin,
                             float sizeFracMax,
                             RansacResult &out,
                             bool verbose)
{
    try {
        PoseCandidateSampler sampler(cfg, target, diag1, diag2, cameraIntrinsics, imageSize, sizeFracMin, sizeFracMax);
        if (cfg.samplerRandomSeed != 0) {
            sampler.seedRandom(cfg.samplerRandomSeed);
        }
        else {
            sampler.seedRandom(time(nullptr));
        }
        std::vector<cv::Point2f> proj1, proj2;
        const int maxIters = std::max(1, cfg.ransacMaxIters);
        const int targetMax = static_cast<int>(target.diag1.size() + target.diag2.size());
        const int earlyStopAt = (cfg.earlyStopPercent > 0) ? (targetMax * cfg.earlyStopPercent / 100) : std::numeric_limits<int>::max();

        if (verbose) {
            std::cout << "earlyStopAt = " << earlyStopAt << ", targetMax = " << targetMax
                      << "(diag1: " << target.diag1.size() << ", diag2: " << target.diag2.size()
                      << ", cfg.earlyStopPercent: " << cfg.earlyStopPercent << ")" << std::endl;
        }

        out.inliers = 0;
        out.iterations = 0;
        std::vector<int> markerInlierIndices1, markerInlierIndices2;
        std::vector<int16_t> nearbyMaskInliers1, nearbyMaskInliers2;
        std::vector<cv::Point2f> targetInliers, photoInliers;

        auto startTime = std::chrono::high_resolution_clock::now();

        const int markerSpaceIterations = 100;

        for (int it = 0; it < maxIters; ++it) {
            out.iterations = it + 1;

            cv::Matx33d H;
            bool flippedDiags = false;
            bool rot180 = false;
            if (!sampler.generate(H, flippedDiags, rot180))
                continue;
            
            if (markerSpaceIterations > 0) {
                cv::Matx33d H_inv = H.inv();
                unprojectedPoints1.clear();
                unprojectedPoints2.clear();
                const auto& photoPoints1 = flippedDiags ? diag2.points : diag1.points;
                const auto& photoPoints2 = flippedDiags ? diag1.points : diag2.points;
                cv::perspectiveTransform(photoPoints1, unprojectedPoints1, H_inv);
                cv::perspectiveTransform(photoPoints2, unprojectedPoints2, H_inv);


                std::vector<cv::Point2i> scaledUnprojectedPoints1, scaledUnprojectedPoints2;
                scaledUnprojectedPoints1.resize(unprojectedPoints1.size());
                scaledUnprojectedPoints2.resize(unprojectedPoints2.size());

                if (verbose) {
                    //std::cout << "targetNearbyMask1.size() = " << targetNearbyMask1.size() << std::endl;
                    //std::cout << "targetNearbyMask2.size() = " << targetNearbyMask2.size() << std::endl;
                }

                float bestScale = 1;
                cv::Point2i bestOffset2D;
                int bestScore = 0;
                for (int i = 0; i < markerSpaceIterations; ++i) {

                    // Each iteration, guess one random point correspondence and align the
                    // points. Keep only the most similar alignment as homography matrix.
                    // This is done in marker space (axis aligned, no perspective, int math),
                    // so it is much faster than the full 3D inlier check.
                    // We do fewer but better outer iterations.

                    float scale = 0.5f + (rand() % 1000) / 1000.0f;
                    for(int j = 0; j < unprojectedPoints1.size(); ++j) {
                        scaledUnprojectedPoints1[j].x = static_cast<int>(unprojectedPoints1[j].x * scale);
                        scaledUnprojectedPoints1[j].y = static_cast<int>(unprojectedPoints1[j].y * scale);
                    }
                    for(int j = 0; j < unprojectedPoints2.size(); ++j) {
                        scaledUnprojectedPoints2[j].x = static_cast<int>(unprojectedPoints2[j].x * scale);
                        scaledUnprojectedPoints2[j].y = static_cast<int>(unprojectedPoints2[j].y * scale);
                    }

                    cv::Point2i targetPoint, unprojectedPoint;
                    const bool correspondenceDiag = rand() % 2 == 0;
                    if (correspondenceDiag) {
                        targetPoint = target.diag1[rand() % target.diag1.size()];
                        unprojectedPoint = scaledUnprojectedPoints1[rand() % unprojectedPoints1.size()];
                    }
                    else {
                        targetPoint = target.diag2[rand() % target.diag2.size()];
                        unprojectedPoint = scaledUnprojectedPoints2[rand() % unprojectedPoints2.size()];
                    }
                    const auto& offset2D = targetPoint - unprojectedPoint;
                    for(int j = 0; j < scaledUnprojectedPoints1.size(); ++j) {
                        scaledUnprojectedPoints1[j].x += offset2D.x;
                        scaledUnprojectedPoints1[j].y += offset2D.y;
                    }
                    for(int j = 0; j < scaledUnprojectedPoints2.size(); ++j) {
                        scaledUnprojectedPoints2[j].x += offset2D.x;
                        scaledUnprojectedPoints2[j].y += offset2D.y;
                    }
                    int inlierCount1 = countInliers(
                        scaledUnprojectedPoints1, target.nearbyMask1,
                        target.bitmatrix.cols, target.bitmatrix.rows
                    );

                    int inlierCount2 = countInliers(
                        scaledUnprojectedPoints2, target.nearbyMask2,
                        target.bitmatrix.cols, target.bitmatrix.rows
                    );

                    if (inlierCount1 + inlierCount2 > bestScore) {
                        bestScore = inlierCount1 + inlierCount2;
                        bestOffset2D = offset2D;
                        bestScale = scale;
                    }
                }

                /*
                if (verbose) {
                    std::cout << "bestScore = " << bestScore << std::endl;
                    std::cout << "bestOffset2D = " << bestOffset2D << std::endl;
                    std::cout << "bestScale = " << bestScale << std::endl;
                }
                */
                cv::Matx33d offsetMat = cv::Matx33d::eye();
                offsetMat(0,0) = bestScale;
                offsetMat(1,1) = bestScale;
                offsetMat(0,2) = bestOffset2D.x;
                offsetMat(1,2) = bestOffset2D.y;
                H_inv = offsetMat * H_inv;
                H = H_inv.inv();
            }

            // Project families
            projectWithH(target.diag1, H, proj1);
            projectWithH(target.diag2, H, proj2);

            // Score via NearbyMask one-to-one
            const auto& labelsForProj1 = flippedDiags ? label2 : label1;
            const auto& labelsForProj2 = flippedDiags ? label1 : label2;

            int s1 = countInliersOneToOne(proj1, labelsForProj1, maskW, maskH, markerInlierIndices1, nearbyMaskInliers1);
            int s2 = countInliersOneToOne(proj2, labelsForProj2, maskW, maskH, markerInlierIndices2, nearbyMaskInliers2);
            int score = s1 + s2;
            if (score < 4) {
                // Not good enough, skip ahead.
                continue;
            }

            const auto& diagsForProj1 = flippedDiags ? diag2 : diag1;
            const auto& diagsForProj2 = flippedDiags ? diag1 : diag2;

            // Iteratively refine pose using only inliers
            for (int refineIter = 0; refineIter < cfg.maxInnerRefinements; ++refineIter) {
                targetInliers.clear();
                photoInliers.clear();
                for (int i = 0; i < markerInlierIndices1.size(); ++i) {
                    targetInliers.push_back(target.diag1[markerInlierIndices1[i]]);
                }
                for (int i = 0; i < markerInlierIndices2.size(); ++i) {
                    targetInliers.push_back(target.diag2[markerInlierIndices2[i]]);
                }
                for (int i = 0; i < nearbyMaskInliers1.size(); ++i) {
                    int label = nearbyMaskInliers1[i];
                    if (label >= 0) {
                        photoInliers.push_back(diagsForProj1.points[i]);
                    }
                }
                for (int i = 0; i < nearbyMaskInliers2.size(); ++i) {
                    int label = nearbyMaskInliers2[i];
                    if (label >= 0) {
                        photoInliers.push_back(diagsForProj2.points[i]);
                    }
                }

                int numDiag1Pairs = static_cast<int>(markerInlierIndices1.size());
                int numDiag2Pairs = static_cast<int>(markerInlierIndices2.size());

                cv::Mat newH;
                if (cfg.useFindHomographyFast) {
                    bool found = findHomographyFast(targetInliers, photoInliers, newH);
                    if (!found)
                        continue;
                }
                else {
                    newH = cv::findHomography(targetInliers, photoInliers, 0);
                    if (newH.empty())
                        continue;
                } 

                projectWithH(target.diag1, newH, proj1);
                projectWithH(target.diag2, newH, proj2);
                markerInlierIndices1.clear();
                markerInlierIndices2.clear();
                nearbyMaskInliers1.clear();
                nearbyMaskInliers2.clear();
                const int newS1 = countInliersOneToOne(proj1, labelsForProj1, maskW, maskH, markerInlierIndices1, nearbyMaskInliers1);
                const int newS2 = countInliersOneToOne(proj2, labelsForProj2, maskW, maskH, markerInlierIndices2, nearbyMaskInliers2);
                const int newScore = newS1 + newS2;

                if (newScore > score) {
                    H = newH;
                    score = newScore;
                    s1 = newS1;
                    s2 = newS2;
                }
                else {
                    break;
                }
            }

            const bool improved = (score > out.inliers);
            if (improved) {
                out.inliers = score;
                out.H = H;
                out.flipDiags = flippedDiags;
                out.rot180 = rot180;
                //std::cout << "Ransac improved: score = " << score << " (diag1: " << s1 << ", diag2: " << s2 << ")" << std::endl;

                if (out.inliers >= earlyStopAt) {
                    if (verbose) {
                        std::cout << "Ransac stopping early: reached " << out.inliers << " inliers" << std::endl;
                    }
                    break;
                }
            }
        }

        // Final refinement with all inliers
        if (out.inliers < 4) {
            if (verbose) {
                std::cout << "Ransac found no good pose: not enough inliers" << std::endl;
            }
            return false;
        }

        if (cfg.finalRefinePnP) {
            projectWithH(target.diag1, out.H, proj1);
            projectWithH(target.diag2, out.H, proj2);

            const auto& labelsForProj1 = out.flipDiags ? label2 : label1;
            const auto& labelsForProj2 = out.flipDiags ? label1 : label2;

            const int finalS1 = countInliersOneToOne(proj1, labelsForProj1, maskW, maskH, markerInlierIndices1, nearbyMaskInliers1);
            const int finalS2 = countInliersOneToOne(proj2, labelsForProj2, maskW, maskH, markerInlierIndices2, nearbyMaskInliers2);
            const int finalScore = finalS1 + finalS2;
            //std::cout << "Final refinement with all inliers (score: " << finalScore << ")" << std::endl;

            const auto& diagsForProj1 = out.flipDiags ? diag2 : diag1;
            const auto& diagsForProj2 = out.flipDiags ? diag1 : diag2;
            std::vector<cv::Point3f> targetInliers3D;
            targetInliers3D.reserve(out.inliers);
            photoInliers.clear();
            for (int i = 0; i < markerInlierIndices1.size(); ++i) {
                const auto& p = target.diag1[markerInlierIndices1[i]];
                targetInliers3D.push_back(cv::Point3f(
                    p.x / target.bitmatrix.cols - 0.5f,
                    p.y / target.bitmatrix.rows - 0.5f,
                    0.0f));
            }
            for (int i = 0; i < markerInlierIndices2.size(); ++i) {
                const auto& p = target.diag2[markerInlierIndices2[i]];
                targetInliers3D.push_back(cv::Point3f(
                    p.x / target.bitmatrix.cols - 0.5f,
                    p.y / target.bitmatrix.rows - 0.5f,
                    0.0f));
            }
            for (int i = 0; i < nearbyMaskInliers1.size(); ++i) {
                const auto& p = diagsForProj1.points[nearbyMaskInliers1[i]];
                photoInliers.push_back(p);
            }
            for (int i = 0; i < nearbyMaskInliers2.size(); ++i) {
                const auto& p = diagsForProj2.points[nearbyMaskInliers2[i]];
                photoInliers.push_back(p);
            }
            const bool poseFound = cv::solvePnP(targetInliers3D, photoInliers, cameraIntrinsics, cv::noArray(), out.rvec, out.tvec, false, cv::SOLVEPNP_SQPNP);
            if (poseFound) {
                out.tvec *= target.sideLengthMeters;

                std::vector<cv::Point2f> targetInliersMarkerSpace(targetInliers3D.size());
                for (int i = 0; i < targetInliers3D.size(); ++i) {
                    targetInliersMarkerSpace[i] = cv::Point2f(
                        (targetInliers3D[i].x + 0.5f) * target.bitmatrix.cols,
                        (targetInliers3D[i].y + 0.5f) * target.bitmatrix.rows
                    );
                }
                out.H = cv::findHomography(targetInliersMarkerSpace, photoInliers, 0);
            }
            else {
                if (verbose) {
                    std::cout << "Final refinement with all inliers failed to find pose" << std::endl;
                }
                return false;
            }
        }
        else {
            cv::Vec3d rvec, tvec;
            std::vector<cv::Point3f> objectCorners = calcObjectSpaceCorners(target.sideLengthMeters);
            std::vector<cv::Point2f> targetCorners = calcTargetSpaceCorners(target.bitmatrix.cols);
            std::vector<cv::Point2f> photoCorners;
            projectWithH(targetCorners, out.H, photoCorners);

            bool foundPose = cv::solvePnP(objectCorners, photoCorners, cameraIntrinsics, cv::noArray(), out.rvec, out.tvec, false, cv::SOLVEPNP_IPPE_SQUARE);
            if (!foundPose) {
                if (verbose) {
                    std::cout << "Ransac found no good pose in final solvePnP" << std::endl;
                }
                return false;
            }
        }

        auto endTime = std::chrono::high_resolution_clock::now();
        auto duration = std::chrono::duration_cast<std::chrono::microseconds>(endTime - startTime).count();

        double timePerIter = (duration * 100) / out.iterations / 100.0;
        std::cout << "Ransac took " << (duration / 1000) << " ms (" << out.iterations << " iterations x " << timePerIter << " µs)" << std::endl;

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
                        float sizeFracMin,
                        float sizeFracMax,
                        cv::Matx33d &outH,
                        Pose &outPose,
                        bool &outFlipDiags,
                        bool &outRot180,
                        int &outInliers,
                        int &outIterations,
                        bool debug)
{
    try {
        // Build NearbyMask label maps
        std::vector<int16_t> nearbyMask1, nearbyMask2;

        if (!buildNearbyMask(gray.size(), diag1.points, cfg.inlierRadiusPx, nearbyMask1)) {
            std::cout << "buildNearbyMask diag1 failed" << std::endl;
            return false;
        }

        if (!buildNearbyMask(gray.size(), diag2.points, cfg.inlierRadiusPx, nearbyMask2)) {
            std::cout << "buildNearbyMask diag2 failed" << std::endl;
            return false;
        }

        if (debug) {
            std::cout << "nearbyMask1.size() = " << nearbyMask1.size() << std::endl;
            std::cout << "nearbyMask2.size() = " << nearbyMask2.size() << std::endl;
            plotNearbyMask(gray, nearbyMask1, "nearbyMask1.jpg");
            plotNearbyMask(gray, nearbyMask2, "nearbyMask2.jpg");
        }

        RansacResult result;
        bool success = ransacHomography(
            cfg, target, diag1, diag2, cameraIntrinsics,
            gray.size(), nearbyMask1, nearbyMask2, gray.cols, gray.rows,
            sizeFracMin, sizeFracMax, result, debug
        );

        if (!success) {
            std::cout << "ransacHomography failed" << std::endl;
            return false;
        }

        outH = result.H;
        outPose.rvec = result.rvec;
        outPose.tvec = result.tvec;
        outFlipDiags = result.flipDiags;
        outRot180 = result.rot180;
        outInliers = result.inliers;
        outIterations = result.iterations;
        if (debug) {
            std::cout << "estimateWithRansac succeed with " << outInliers
                    << " inliers after " << outIterations << " iterations"
                    << std::endl;
        }

        return true;
    } catch (const std::exception &e) {
        std::cerr << "estimateWithRansac exception: " << e.what() << std::endl;
        return false;
    }
}

} // namespace BitMatrixTracker
} // namespace psm
