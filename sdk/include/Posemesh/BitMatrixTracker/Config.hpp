#pragma once

#include <opencv2/core.hpp>
#include <string>

namespace psm {
namespace BitMatrixTracker {

struct Config {
    int tileSizePx;                   // default: 40
    float peakThreshold;              // default: 0.25
    int tileHistogramBins;            // default: 32
    float minContrast;                // default: 0.30
    bool validateSubtiles;            // default: true
    int minClusterSideLengthTiles;    // default: 2
    int minClusterValidTilesCount;    // default: 4

    std::string cornerNetWeightsPath; // default: "cornernet.bin"
    float confidenceThreshold;        // default: 0.80
    int angleHistBins;                // default: 30
    float angleKeepDegLoose;          // default: 20.0f
    float angleKeepDegStrict;         // default: 15.0f
    float orthogonalityToleranceDeg;  // default: 10.0f
    float angleJitterDeg;             // default: 2.0f

    // RANSAC / inliers / collapse
    int ransacMaxIters;                 // default: 50000
    int maxInnerRefinements;            // default: 4
    bool useFindHomographyFast;         // default: true
    float inlierRadiusPx;               // default: 6.0f
    float collapseRadiusPx;             // default: 2.0f
    //float sizeFracMin;                // default: 0.08f
    //float sizeFracMax;                // default: 0.50f
    //int sizeFracBins;                 // default: 10
    int earlyStopPercent;               // default: 70
    //float convergenceStrength;        // default: 1.0f

    bool finalRefinePnP;              // default: true
};

// Returns Python-matching defaults.
const Config &defaultConfig();

} // namespace BitMatrixTracker
} // namespace psm