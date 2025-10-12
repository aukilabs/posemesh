#include <limits>
#include <vector>
#include <iostream>
#include <opencv2/core.hpp>

namespace psm {
namespace BitMatrixTracker {

// Plan (agreed): one-to-one matching per iteration.
// 1) Build collapsed centers per family (done in Angles.cpp step).
// 2) For each family, make a label map (int16_t) with nearest center index within inlierRadiusPx.
//    Use a temporary bestDistSq (uint16_t) to break ties. Reuse buffers between frames.
// 3) During RANSAC, maintain a small used[] bitset to enforce one-to-one claiming of detections.

// TODO: implement helpers to allocate/update label maps for a given ROI.

} // namespace BitMatrixTracker
} // namespace psm