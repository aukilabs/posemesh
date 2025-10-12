#pragma once

#include <string>
#include <vector>
#include <opencv2/core.hpp>


namespace psm {
namespace BitMatrixTracker {


// Small hand-coded MLP used by the detector (5x5 -> 32 -> 24 -> {conf, rot}).
// We keep weights in simple row-major contiguous arrays for cache-friendly matvecs.
struct CornerNetWeights {
// Layer 1: 25 -> 32
std::vector<float> W1; // size 32*25, row-major: row i has 25 weights
std::vector<float> b1; // size 32
// Layer 2: 32 -> 24
std::vector<float> W2; // size 24*32
std::vector<float> b2; // size 24
// Heads: 24 -> 1 (conf) and 24 -> 1 (rot)
std::vector<float> Wc; // size 1*24
float bc {0.0f};
std::vector<float> Wr; // size 1*24
float br {0.0f};


bool isValid() const;
};


// Loads weights from a simple custom binary file or from embedded arrays.
bool loadCornerNetWeightsFromFile(const std::string &path, CornerNetWeights &out);


// Forward pass on a 5x5 patch in uint8 or float [0..1].
// angleDeg is returned in [0, 90).
void runCornerNet5x5U8(const uint8_t *patch5x5, const CornerNetWeights &w, float &conf, float &angleDeg);
void runCornerNet5x5F32(const float *patch5x5, const CornerNetWeights &w, float &conf, float &angleDeg);


} // namespace BitMatrixTracker
} // namespace psm