#pragma once

#include "Types.hpp"

namespace psm {
namespace BitMatrixTracker {

struct QrEncodeParams { /* reserved for version/ECC options if needed */ };

// Builds Target by encoding content to a bitmatrix, then extracting diag features.
bool makeTargetFromContent(const std::string &content,
                           float sideLengthMeters,
                           Target &outTarget,
                           const QrEncodeParams *enc = nullptr);

// Builds Target directly from a provided bitmatrix.
bool makeTargetFromBitmatrix(const cv::Mat1b &bitmatrix,
                             float sideLengthMeters,
                             Target &outTarget);

} // namespace BitMatrixTracker
} // namespace psm