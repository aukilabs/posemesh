#include "Posemesh/BitMatrixTracker/Config.hpp"

namespace psm {
namespace BitMatrixTracker {

static Config s_defaultConfig { /* all fields populated below */ };

const Config &defaultConfig()
{
    // Values mirror the Python prototype exactly for v1.
    s_defaultConfig.tileSizePx = 40;
    s_defaultConfig.peakThreshold = 0.25f;
    s_defaultConfig.tileHistogramBins = 32;
    s_defaultConfig.minContrast = 0.30f;
    s_defaultConfig.validateSubtiles = true;
    s_defaultConfig.minClusterSideLengthTiles = 2;
    s_defaultConfig.minClusterValidTilesCount = 4;

    s_defaultConfig.cornerNetWeightsPath = "cornernet.bin";
    s_defaultConfig.confidenceThreshold = 0.80f;
    s_defaultConfig.angleHistBins = 30;
    s_defaultConfig.angleKeepDegLoose = 20.0f;
    s_defaultConfig.angleKeepDegStrict = 15.0f;
    s_defaultConfig.orthogonalityTolDeg = 10.0f;
    s_defaultConfig.angleJitterDeg = 2.0f;

    s_defaultConfig.ransacMaxIters = 50000;
    s_defaultConfig.inlierRadiusPx = 6.0f;
    s_defaultConfig.collapseRadiusPx = 2.0f;
    s_defaultConfig.sizeFracMin = 0.08f;
    s_defaultConfig.sizeFracMax = 0.50f;
    s_defaultConfig.sizeFracBins = 10;
    s_defaultConfig.earlyStopPercent = 70;
    s_defaultConfig.convergenceStrength = 1.0f;

    s_defaultConfig.finalRefinePnP = true;

    return s_defaultConfig;
}

} // namespace BitMatrixTracker
} // namespace psm