#include <iostream>
#include <opencv2/core.hpp>
#include <opencv2/imgproc.hpp>
#include <opencv2/objdetect.hpp>   // if we use OpenCV QR encode/decode later
#include "Posemesh/BitMatrixTracker/Precompute.hpp"

namespace psm {
namespace BitMatrixTracker {

static inline uint8_t bin01(uint8_t v) { return v ? 1 : 0; }

// Scan the QR bitmatrix for alternating 2x2 patterns and emit their centers
// into two diagonal families:
//  - diag1 corresponds to [1 0; 0 1]
//  - diag2 corresponds to [0 1; 1 0]
// Coordinates are in module units, with centers at integer (x+1, y+1).
static bool extractDiagonalFeatures(const cv::Mat1b &bitmatrix,
                                    std::vector<cv::Point2f> &diag1,
                                    std::vector<cv::Point2f> &diag2)
{
    try {
        CV_Assert(bitmatrix.channels() == 1);
        const int rows = bitmatrix.rows;
        const int cols = bitmatrix.cols;
        if (rows < 2 || cols < 2) {
            std::cerr << "extractDiagonalFeatures: bitmatrix too small" << std::endl;
            return false;
        }
        diag1.clear();
        diag2.clear();

        for (int y = 0; y < rows - 1; ++y) {
            const uint8_t *r0 = bitmatrix.ptr<uint8_t>(y);
            const uint8_t *r1 = bitmatrix.ptr<uint8_t>(y + 1);
            for (int x = 0; x < cols - 1; ++x) {
                // Read 2x2 block
                const uint8_t b00 = bin01(r0[x]);
                const uint8_t b01 = bin01(r0[x + 1]);
                const uint8_t b10 = bin01(r1[x]);
                const uint8_t b11 = bin01(r1[x + 1]);

                // Alternating check: b00==b11, b01==b10, and b00 != b01
                const bool alternating = (b00 == b11) && (b01 == b10) && (b00 != b01);
                if (!alternating)
                    continue;

                // Center of the 2x2 block in module units
                const float cx = static_cast<float>(x + 1);
                const float cy = static_cast<float>(y + 1);

                if (b00 == 1) {
                    // [1 0; 0 1]
                    diag1.emplace_back(cx, cy);
                } else {
                    // [0 1; 1 0]
                    diag2.emplace_back(cx, cy);
                }
            }
        }
        return true;
    } catch (const std::exception &e) {
        std::cerr << "extractDiagonalFeatures exception: " << e.what() << std::endl;
        return false;
    }
}

bool makeTargetFromContent(const std::string &content,
                           float sideLengthMeters,
                           Target &outTarget,
                           const QrEncodeParams *enc)
{
    try {
        (void)enc;
#if CV_VERSION_MAJOR >= 4
        // NOTE: OpenCV's QRCodeEncoder produces a raster image, not a module grid.
        // Converting that to a precise bitmatrix robustly is non-trivial.
        // For now, require callers to provide a bitmatrix directly.
        std::cerr << "makeTargetFromContent: not implemented yet; please supply a bitmatrix.\n";
        (void)content; (void)sideLengthMeters; (void)outTarget;
        return false;
#else
        std::cerr << "makeTargetFromContent: OpenCV version not supported for encoding.\n";
        return false;
#endif
    } catch (const std::exception &e) {
        std::cerr << "makeTargetFromContent exception: " << e.what() << std::endl;
        return false;
    }
}

bool makeTargetFromBitmatrix(const cv::Mat1b &bitmatrix,
                             float sideLengthMeters,
                             Target &outTarget)
{
    try {
        if (bitmatrix.empty()) {
            std::cerr << "makeTargetFromBitmatrix: empty bitmatrix" << std::endl;
            return false;
        }
        // Normalize to {0,1}
        cv::Mat1b B;
        if (bitmatrix.type() != CV_8U) bitmatrix.convertTo(B, CV_8U);
        else B = bitmatrix.clone();
        cv::threshold(B, B, 0, 1, cv::THRESH_BINARY);

        outTarget.bitmatrix = B;
        outTarget.sideLengthMeters = sideLengthMeters;
        outTarget.diag1.clear();
        outTarget.diag2.clear();
        if (!extractDiagonalFeatures(outTarget.bitmatrix, outTarget.diag1, outTarget.diag2))
            return false;
        return true;
    } catch (const std::exception &e) {
        std::cerr << "makeTargetFromBitmatrix exception: " << e.what() << std::endl;
        return false;
    }
}

} // namespace BitMatrixTracker
} // namespace psm
