#pragma once

#include <opencv2/core.hpp>
#include <string>
#include <vector>

namespace psm {
namespace BitMatrixTracker {

struct Target {
    std::string id;                 // content/ID (optional)
    cv::Mat1b bitmatrix;            // {0,1} or {0,255}
    float sideLengthMeters;         // known metric size of outer square

    // Feature coords in canonical target plane (units consistent with PnP setup)
    std::vector<cv::Point2f> diag1; // family near dominant angle
    std::vector<cv::Point2f> diag2; // perpendicular family
};

struct Cluster {
    cv::Rect tileBounds;            // bounds in tile indices
    cv::Rect pixelBounds;           // expanded bounds in pixel coords
    cv::Mat1b tileMask;             // ROI mask in tile space (1=valid)
};

struct Detections {
    std::vector<cv::Point2f> points;    // image pixel centers
    std::vector<float> anglesDeg;       // modulo 90°
};

struct Diagnostics {
    int validTileCount {0};
    int clusterCount {0};
    int bestClusterIndex {0};
    int rawCorners {0};
    int keptCornersLoose {0};
    int keptCornersStrict {0};
    int inliersBest {0};
    int ransacIterations {0};
};

struct Pose {
    cv::Vec3d rvec;  // Rodrigues, camera frame
    cv::Vec3d tvec;  // meters
};

} // namespace BitMatrixTracker
} // namespace psm