#include <vector>
#include <cmath>
#include <algorithm>
#include <iostream>
#include "Posemesh/BitMatrixTracker/Config.hpp"
#include "Posemesh/BitMatrixTracker/Types.hpp"

namespace psm {
namespace BitMatrixTracker {

// TODO: build global histogram over cluster detections, smooth (radius=2), find peak.
// Keep within ±angleKeepDegLoose, recompute peak, keep ±angleKeepDegStrict.
// Split into two families (peak and peak+90). Then collapse by collapseRadiusPx using grid + union-find.

} // namespace BitMatrixTracker
} // namespace psm