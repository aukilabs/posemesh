#include "Posemesh/BitMatrixTracker/Geometry.hpp"

#include <opencv2/core.hpp>
#include <opencv2/calib3d.hpp>
#include <iostream>
#include <random>

namespace psm {
namespace BitMatrixTracker {

bool normalizeVec2(const cv::Vec2d& vec, cv::Vec2d& out, float eps)
{
    double norm = cv::norm(vec);
    if (norm < eps)
        return false;
    out = vec / norm;
    return true;
}


// Project 2D target-plane points using homography H -> pixel integer coords.
void projectWithH(const std::vector<cv::Point2f> &src,
                         const cv::Matx33d &H,
                         std::vector<cv::Point2i> &dstInt)
{
    dstInt.resize(src.size());
    for (size_t i = 0; i < src.size(); ++i) {
        const double x = src[i].x, y = src[i].y;
        const double X = H(0,0)*x + H(0,1)*y + H(0,2);
        const double Y = H(1,0)*x + H(1,1)*y + H(1,2);
        const double Z = H(2,0)*x + H(2,1)*y + H(2,2);
        const double ix = X / Z;
        const double iy = Y / Z;
        dstInt[i].x = static_cast<int>(std::lround(ix));
        dstInt[i].y = static_cast<int>(std::lround(iy));
    }
}

cv::Vec2d directionVec(double angleDeg)
{
    const double rad = angleDeg * static_cast<double>(CV_PI) / 180.0;
    return cv::Vec2d(std::cos(rad), std::sin(rad));
}

double signedAngle2D(const cv::Vec2d &a, const cv::Vec2d &b)
{
    double rad = std::atan2(a[0] * b[1] - a[1] * b[0], a.dot(b));
    return rad * (180.0 / static_cast<double>(CV_PI));
}

// Only used in this file
static inline double _solve_L1_from_alpha(
    double u0, double v0, 
    double dx, double dy, 
    double fx, double fy, 
    double cx, double cy, 
    double alpha)
{
    // Solve || c̃ + L * ã ||^2 = tan^2(alpha) for L (forward root)
    // c̃ = ((u0-cx)/fx, (v0-cy)/fy)
    // ã = (dx/fx, dy/fy)
    // Returns L1 if L1 >= 0, else L2.
    double R = std::tan(alpha);
    double ux = (u0 - cx) / fx;
    double uy = (v0 - cy) / fy;
    double ax = dx / fx;
    double ay = dy / fy;

    double A = ax * ax + ay * ay;
    double B = 2.0 * (ux * ax + uy * ay);
    double C = ux * ux + uy * uy - R * R;

    double disc = B * B - 4.0 * A * C;
    if (disc < 0.0)
        disc = 0.0;
    double sqrtD = std::sqrt(disc);
    double inv2A = 0.5 / A;

    double L1 = (-B + sqrtD) * inv2A;
    double L2 = (-B - sqrtD) * inv2A;
    return L1 >= 0.0 ? L1 : L2;
}

// Only used in this file
static inline double _solve_L2_from_orthogonality(
    double u0, double v0,
    double drx, double dry,
    double dux, double duy,
    double fx, double fy,
    double cx, double cy,
    double L1)
{
    // Compute L2 so (K^-1 v1)·(K^-1 v2)=0
    double ux0 = (u0 - cx) / fx;
    double uy0 = (v0 - cy) / fy;
    double arx = drx / fx;
    double ary = dry / fy;
    double aux = dux / fx;
    double auy = duy / fy;
    double A = arx * aux + ary * auy;
    double B1 = ux0 * aux + uy0 * auy;
    double B2 = ux0 * arx + uy0 * ary;
    double C = ux0 * ux0 + uy0 * uy0 + 1.0;
    double denom = A * L1 + B2;
    // caller should guard |denom| small
    return -(B1 * L1 + C) / denom;
}

bool homographyFromPointAndDirs(
    const cv::Point2f& markerPoint,
    const cv::Point2f& photoPoint,
    const cv::Vec2d& rightInImage,
    const cv::Vec2d& upInImage,
    int imageWidth,
    int imageHeight,
    double bitMatrixSize,
    const cv::Matx33d& K,
    double sizeFrac,
    cv::Matx33d& homographyOut,
    std::mt19937& rng,
    bool enforceRightHanded,
    double eps
)
{
    using Vec2 = cv::Vec2d;
    using Matx33 = cv::Matx33d;

    // Normalize input vectors robustly
    Vec2 rightDir, upDir;    
    if (!normalizeVec2(rightInImage, rightDir, eps)) { 
        std::cerr << "homographyFromPointAndDirs: failed to normalize rightInImage (≈0 length)" << std::endl;
        return false;
    }
    if (!normalizeVec2(upInImage, upDir, eps)) {
        std::cerr << "homographyFromPointAndDirs: failed to normalize upInImage (≈0 length)" << std::endl;
        return false;
    }
    
    if (std::abs(rightDir.dot(upDir)) > 0.999) {
        //std::cerr << "homographyFromPointAndDirs: rightInImage and upInImage are collinear" << std::endl;
        return false;
    }
    
    double fx = K(0,0);
    double fy = K(1,1);
    double cx = K(0,2);
    double cy = K(1,2);

    const double minTiltAngle = 50.0f; // Viewing from side
    const double maxTiltAngle = 90.0f; // Straight on
    double tiltAngleDeg = std::uniform_real_distribution<double>(minTiltAngle, maxTiltAngle)(rng);
    double tiltAngleRad = tiltAngleDeg * M_PI / 180.0;

    double L1 = _solve_L1_from_alpha(
        photoPoint.x, photoPoint.y,
        rightDir[0], rightDir[1],
        fx, fy, cx, cy,
        tiltAngleRad
    );

    double L2 = _solve_L2_from_orthogonality(
        photoPoint.x, photoPoint.y,
        rightDir[0], rightDir[1],
        upDir[0], upDir[1],
        fx, fy, cx, cy,
        L1
    );

    // Compute vanishing points
    double v1x = photoPoint.x + L1 * rightDir[0];
    double v1y = photoPoint.y + L1 * rightDir[1];
    double v2x = photoPoint.x + L2 * upDir[0];
    double v2y = photoPoint.y + L2 * upDir[1];

    // Back-project vanishing points to camera directions
    double b1x = (v1x - cx) / fx;
    double b1y = (v1y - cy) / fy;
    double b1z = 1.0;

    double b2x = (v2x - cx) / fx;
    double b2y = (v2y - cy) / fy;
    double b2z = 1.0;

    // Orthonormalize → r1, r2, r3 (Gram–Schmidt)
    double n1 = std::sqrt(b1x * b1x + b1y * b1y + b1z * b1z);
    double n2 = std::sqrt(b2x * b2x + b2y * b2y + b2z * b2z);
    if (n1 < 1e-12 || n2 < 1e-12) {
        homographyOut = cv::Matx33d::zeros();
        std::cerr << "homographyFromPointAndDirs: n1 < 1e-12 || n2 < 1e-12" << std::endl;
        return false;
    }
    double r1x = b1x / n1, r1y = b1y / n1, r1z = b1z / n1;
    double dot12 = r1x * b2x + r1y * b2y + r1z * b2z;
    double b2ox = b2x - r1x * dot12;
    double b2oy = b2y - r1y * dot12;
    double b2oz = b2z - r1z * dot12;
    double n2o = std::sqrt(b2ox * b2ox + b2oy * b2oy + b2oz * b2oz);
    if (n2o < 1e-6) {
        homographyOut = cv::Matx33d::zeros();
        std::cerr << "homographyFromPointAndDirs: n2o < 1e-6" << std::endl;
        return false;
    }
    double r2x = b2ox / n2o, r2y = b2oy / n2o, r2z = b2oz / n2o;

    // Right-handed
    double r3x = r1y * r2z - r1z * r2y;
    double r3y = r1z * r2x - r1x * r2z;
    double r3z = r1x * r2y - r1y * r2x;
    if (enforceRightHanded) {
        double det = r1x * (r2y * r3z - r2z * r3y)
                   - r1y * (r2x * r3z - r2z * r3x)
                   + r1z * (r2x * r3y - r2y * r3x);
        if (det < 0.0) {
            r2x = -r2x; r2y = -r2y; r2z = -r2z;
            r3x = -r3x; r3y = -r3y; r3z = -r3z;
        }
    }

    // Scale from apparent size prior
    double minDim = std::min(imageWidth, imageHeight);
    double step_px = (sizeFrac * minDim) / bitMatrixSize;
    double f_eff = 0.5 * (fx + fy);
    double s = step_px / f_eff;

    // Columns h1=K(s r1), h2=K(s r2)
    double sr1x = s * r1x, sr1y = s * r1y, sr1z = s * r1z;
    double sr2x = s * r2x, sr2y = s * r2y, sr2z = s * r2z;
    double h1x = fx * sr1x + cx * sr1z, h1y = fy * sr1y + cy * sr1z, h1z = sr1z;
    double h2x = fx * sr2x + cx * sr2z, h2y = fy * sr2y + cy * sr2z, h2z = sr2z;

    // Translation so (xm, ym) --> (u0, v0)
    double A0 = h1x * markerPoint.x + h2x * markerPoint.y;
    double A1 = h1y * markerPoint.x + h2y * markerPoint.y;
    double B  = h1z * markerPoint.x + h2z * markerPoint.y;
    double c2 = 1.0;
    double c0 = photoPoint.x * (B + c2) - A0;
    double c1 = photoPoint.y * (B + c2) - A1;

    homographyOut = cv::Matx33d::zeros();
    homographyOut(0, 0) = h1x; homographyOut(0, 1) = h2x; homographyOut(0, 2) = c0;
    homographyOut(1, 0) = h1y; homographyOut(1, 1) = h2y; homographyOut(1, 2) = c1;
    homographyOut(2, 0) = h1z; homographyOut(2, 1) = h2z; homographyOut(2, 2) = c2;

    // Cheirality at the correspondence
    double w0 = homographyOut(2, 0) * markerPoint.x + homographyOut(2, 1) * markerPoint.y + homographyOut(2, 2);
    if (w0 <= 0.0) {
        // flip r2
        sr2x = -sr2x;
        sr2y = -sr2y;
        sr2z = -sr2z;
        h2x = fx * sr2x + cx * sr2z;
        h2y = fy * sr2y + cy * sr2z;
        h2z = sr2z;
        A0 = h1x * markerPoint.x + h2x * markerPoint.y;
        A1 = h1y * markerPoint.x + h2y * markerPoint.y;
        B = h1z * markerPoint.x + h2z * markerPoint.y;
        c0 = photoPoint.x * (B + c2) - A0;
        c1 = photoPoint.y * (B + c2) - A1;
        homographyOut(0, 1) = h2x;
        homographyOut(1, 1) = h2y;
        homographyOut(2, 1) = h2z;
        homographyOut(0, 2) = c0;
        homographyOut(1, 2) = c1;

        w0 = homographyOut(2, 0) * markerPoint.x + homographyOut(2, 1) * markerPoint.y + homographyOut(2, 2);
        if (w0 <= 0.0) {
            homographyOut = cv::Matx33d::zeros();
            std::cerr << "homographyFromPointAndDirs: w0 <= 0.0" << std::endl;
            return false;
        }
    }

    return true;
}

} // namespace BitMatrixTracker
} // namespace psm