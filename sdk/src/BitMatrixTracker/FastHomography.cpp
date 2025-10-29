#include "Posemesh/BitMatrixTracker/FastHomography.hpp"
#include <opencv2/core.hpp>
#include <opencv2/core/mat.hpp>
#include <opencv2/core/types.hpp>
#include <opencv2/core.hpp>
#include <opencv2/calib3d.hpp> // for cv::SVD
#include <cmath>
#include <iostream>

namespace psm {
namespace BitMatrixTracker {

static inline void computeNormalizationTransform(const std::vector<cv::Point2f>& pts,
                                                 cv::Matx33d& T,
                                                 std::vector<cv::Point2f>& ptsNorm)
{
    // Hartley normalization:
    // 1. translate so centroid is at origin
    // 2. scale so mean distance to origin is sqrt(2)

    const int n = static_cast<int>(pts.size());
    ptsNorm.resize(n);

    // centroid
    double mx = 0.0;
    double my = 0.0;
    for (int i = 0; i < n; ++i) {
        mx += pts[i].x;
        my += pts[i].y;
    }
    if (n > 0) {
        mx /= n;
        my /= n;
    }

    // mean distance to centroid
    double meanDist = 0.0;
    for (int i = 0; i < n; ++i) {
        double dx = pts[i].x - mx;
        double dy = pts[i].y - my;
        meanDist += std::sqrt(dx*dx + dy*dy);
    }
    if (n > 0) {
        meanDist /= n;
    }

    // scale
    double s;
    if (meanDist > 1e-12)
        s = std::sqrt(2.0) / meanDist;
    else
        s = 1.0; // all points basically identical; fallback

    // build T:
    // [ s 0 -s*mx ]
    // [ 0 s -s*my ]
    // [ 0 0   1   ]
    T = cv::Matx33d(
        s, 0.0, -s*mx,
        0.0, s, -s*my,
        0.0, 0.0, 1.0
    );

    // apply T to all points
    for (int i = 0; i < n; ++i) {
        double x = pts[i].x;
        double y = pts[i].y;
        double X = s * x - s * mx;
        double Y = s * y - s * my;
        ptsNorm[i] = cv::Point2f(static_cast<float>(X),
                                 static_cast<float>(Y));
    }
}

// Solve DLT using SVD on A*h = 0
// srcNorm, dstNorm are corresponding normalized points
// returns Hnorm (3x3) mapping srcNorm -> dstNorm
static inline bool solveDLT(const std::vector<cv::Point2f>& srcNorm,
                            const std::vector<cv::Point2f>& dstNorm,
                            cv::Matx33d& Hnorm)
{
    const int n = static_cast<int>(srcNorm.size());
    if (n < 4)
        return false;

    // Build A: 2n x 9
    // For each correspondence (x,y) -> (u,v):
    // [-x, -y, -1,  0,  0,  0,  x*u, y*u, u]
    // [ 0,  0,  0, -x, -y, -1,  x*v, y*v, v]
    cv::Mat A(2*n, 9, CV_64F);
    for (int i = 0; i < n; ++i) {
        const double x = srcNorm[i].x;
        const double y = srcNorm[i].y;
        const double u = dstNorm[i].x;
        const double v = dstNorm[i].y;

        double* row0 = A.ptr<double>(2*i + 0);
        double* row1 = A.ptr<double>(2*i + 1);

        row0[0] = -x;
        row0[1] = -y;
        row0[2] = -1.0;
        row0[3] = 0.0;
        row0[4] = 0.0;
        row0[5] = 0.0;
        row0[6] = x*u;
        row0[7] = y*u;
        row0[8] = u;

        row1[0] = 0.0;
        row1[1] = 0.0;
        row1[2] = 0.0;
        row1[3] = -x;
        row1[4] = -y;
        row1[5] = -1.0;
        row1[6] = x*v;
        row1[7] = y*v;
        row1[8] = v;
    }

    // Compute SVD of A. We want the smallest singular vector (last row of V^T).
    cv::SVD svd(A, cv::SVD::MODIFY_A | cv::SVD::FULL_UV);

    // Vt is 9x9. Last row of Vt corresponds to smallest singular value.
    if (svd.vt.rows != 9 || svd.vt.cols != 9) {
        std::cerr << "findHomographyFast: unexpected SVD size" << std::endl;
        return false;
    }

    const double* h = svd.vt.ptr<double>(8); // row 8 (0-based)
    // Reshape h into 3x3
    Hnorm = cv::Matx33d(
        h[0], h[1], h[2],
        h[3], h[4], h[5],
        h[6], h[7], h[8]
    );

    return true;
}

bool findHomographyFast(const std::vector<cv::Point2f>& srcPts,
                        const std::vector<cv::Point2f>& dstPts,
                        cv::Mat& outH)
{
    try {
        const int n = static_cast<int>(srcPts.size());
        if (n < 4 || dstPts.size() != srcPts.size()) {
            std::cerr << "findHomographyFast: need >=4 pairs, same length" << std::endl;
            return false;
        }

        // 1. Normalize both point sets
        cv::Matx33d Tsrc, Tdst;
        std::vector<cv::Point2f> srcNorm, dstNorm;
        computeNormalizationTransform(srcPts, Tsrc, srcNorm);
        computeNormalizationTransform(dstPts, Tdst, dstNorm);

        // 2. Solve normalized DLT
        cv::Matx33d Hnorm;
        if (!solveDLT(srcNorm, dstNorm, Hnorm)) {
            std::cerr << "findHomographyFast: DLT failed" << std::endl;
            return false;
        }

        // 3. Denormalize:
        //    H = Tdst^{-1} * Hnorm * Tsrc
        cv::Matx33d TdstInv;
        {
            // Tdst is upper-triangular affine [s 0 tx; 0 s ty; 0 0 1]
            // We can invert analytically. But let's just use cv::invert for clarity once.
            cv::Matx33d Td = Tdst;
            cv::Mat TdInvMat;
            cv::invert(cv::Mat(Td), TdInvMat, cv::DECOMP_LU);
            TdstInv = TdInvMat;
        }

        cv::Matx33d temp = TdstInv * Hnorm;
        cv::Matx33d H    = temp * Tsrc;

        // 4. Normalize so H(2,2) == 1 (or any consistent scale)
        double s = H(2,2);
        if (std::fabs(s) < 1e-12) {
            // fallback: use norm of bottom row
            s = std::sqrt(H(2,0)*H(2,0) + H(2,1)*H(2,1) + H(2,2)*H(2,2));
            if (std::fabs(s) < 1e-12)
                s = 1.0;
        }
        const double invs = 1.0 / s;
        outH = cv::Mat(3, 3, CV_64F);
        outH.at<double>(0,0) = H(0,0)*invs;
        outH.at<double>(0,1) = H(0,1)*invs;
        outH.at<double>(0,2) = H(0,2)*invs;
        outH.at<double>(1,0) = H(1,0)*invs;
        outH.at<double>(1,1) = H(1,1)*invs;
        outH.at<double>(1,2) = H(1,2)*invs;
        outH.at<double>(2,0) = H(2,0)*invs;
        outH.at<double>(2,1) = H(2,1)*invs;
        outH.at<double>(2,2) = H(2,2)*invs;

        return true;
    } catch (const std::exception& e) {
        std::cerr << "findHomographyFast exception: " << e.what() << std::endl;
        return false;
    }
}

} // namespace BitMatrixTracker
} // namespace psm
