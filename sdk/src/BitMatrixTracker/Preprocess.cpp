#include <iostream>
#include <opencv2/imgproc.hpp>
#include "Posemesh/BitMatrixTracker/Config.hpp"
#include "Posemesh/BitMatrixTracker/Types.hpp"

namespace psm {
namespace BitMatrixTracker {

// Given a histogram of pixels per grayscale bin, try finding one major white and black peak bin, with enough contrast.
// This drastically reduces the number of tiles to process early on.
static inline bool calcValidPeaks(
    std::vector<int> histogram,
    int histogramSum,
    int totalPixels,
    float peakThreshold,
    int minBinsApart,
    int &blackPeakBin,
    int &whitePeakBin
) {
    blackPeakBin = -1;
    whitePeakBin = -1;

    // Find black peak from low end
    int sum = 0;
    for (int i = 0; i < histogram.size() / 2; ++i) {
        sum += histogram[i];
        if (static_cast<float>(sum) / totalPixels >= peakThreshold) {
            blackPeakBin = i;
            //std::cout << "blackPeakBin: " << blackPeakBin << std::endl;
            break;
        }
    }
    if (blackPeakBin == -1) {
        //std::cout << "NO black peak" << std::endl;
        return false;
    }

    // Find white peak from high end
    sum = 0;
    for (int i = histogram.size() - 1; i >= histogram.size() / 2; --i) {
        sum += histogram[i];
        if (static_cast<float>(sum) / totalPixels >= peakThreshold) {
            whitePeakBin = i;
            //std::cout << "whitePeakBin: " << whitePeakBin << std::endl;
            break;
        }
    }
    if (whitePeakBin == -1) {
        //std::cout << "NO white peak, sum = " << sum << std::endl;
        return false;
    }
    
    bool valid = whitePeakBin - blackPeakBin >= minBinsApart;
    //std::cout << "valid: " << valid << std::endl;
    return valid;
}

int computeTileHistogram(const cv::Mat &grayTile, std::vector<int> &histogram, int numBins, cv::Rect &R, int pixelStep) {
    for (int i = R.y; i < R.y + R.height; i += pixelStep) {
        for (int j = R.x; j < R.x + R.width; j += pixelStep) {
            int value = grayTile.at<uint8_t>(i, j);
            int bin = value * numBins / 256;
            histogram[bin]++;
        }
    }
    int histogramSum = 0;
    for (int i = 0; i < numBins; i++) {
        histogram[i] *= pixelStep * pixelStep; // Fill in the gaps when not sampling, to get same sum.
        histogramSum += histogram[i];
    }
    return histogramSum;
}

bool normalizeTile(cv::Mat &grayTile, float peakThreshold, float minContrast, int numBins, bool validateSubtiles) {
    int pixelStep = 2;
    std::vector<int> histogram(numBins, 0);
    cv::Rect rect(0, 0, grayTile.cols, grayTile.rows);
    int histogramSum = computeTileHistogram(grayTile, histogram, numBins, rect, pixelStep);
    int totalPixels = grayTile.rows * grayTile.cols;
    if (histogramSum == 0) {
        return false;
    }
    int minBinIndex = -1;
    int maxBinIndex = -1;
    for (int i = 0; i < numBins; i++) {
        if (histogram[i] > 0) {
            minBinIndex = i;
            break;
        }
    }
    for (int i = numBins - 1; i >= 0; i--) {
        if (histogram[i] > 0) {
            maxBinIndex = i;
            break;
        }
    }
    //std::cout << "minBinIndex: " << minBinIndex << ", maxBinIndex: " << maxBinIndex << std::endl;

    int minBinsApart = (minContrast * numBins);
    int blackPeakBin, whitePeakBin;
    bool valid = calcValidPeaks(
        histogram, histogramSum, totalPixels, 
        peakThreshold, minBinsApart,
        blackPeakBin, whitePeakBin
    );
    if (!valid) {
        return false;
    }

    // Check also that each quadrant has valid contrast, to skip tiles where black/white is not evenly distributed.
    // E.g. a mostly white tile with a lot of black in one corner
    int badSubCount = 0;
    if (validateSubtiles) {
        int halfRows = grayTile.cols / 2;
        int halfCols = grayTile.rows / 2;
        cv::Rect subRects[4] = {
            cv::Rect(0, 0, halfCols, halfRows),
            cv::Rect(halfCols, 0, halfCols, halfRows),
            cv::Rect(0, halfRows, halfCols, halfRows),
            cv::Rect(halfCols, halfRows, halfCols, halfRows)
        };
        for (int i = 0; i < 4; i++) {
            std::vector<int> subHistogram(numBins, 0);
            //std::cout << "subRect: " << subRects[i].x << ", " << subRects[i].y << ", " << subRects[i].width << ", " << subRects[i].height << std::endl;
            int subHistogramSum = computeTileHistogram(grayTile, subHistogram, numBins, subRects[i], pixelStep);
            int subWhitePeakBin, subBlackPeakBin;
            bool subValid = calcValidPeaks(
                subHistogram, subHistogramSum, totalPixels / 4,
                peakThreshold, minBinsApart,
                subBlackPeakBin, subWhitePeakBin);
            //std::cout << "subValid: " << subValid << std::endl;
            if (!subValid) {
                badSubCount++;
                if (badSubCount >= 3) {
                    return false;
                }
            }
        }
    }

    float contrast = (whitePeakBin - blackPeakBin) / static_cast<float>(numBins);
    //std::cout << "contrast: " << contrast << std::endl;
    if (contrast < minContrast) {
        // Also already checked inside calcValidPeak
        return false;
    }
    const double scale = 1.0 / contrast;
    const double shift = -blackPeakBin * scale;
    return true;
}

bool computeTileValidityAndNormalized(const cv::Mat &grayTile,
                                      int tileSizePx,
                                      float peakThreshold,
                                      int tileHistogramBins,
                                      float minContrast,
                                      bool validateSubtiles,
                                      cv::Mat1b &outTileMask,
                                      cv::Mat &outNormalized)
{
    try {
        if (grayTile.empty() || grayTile.channels() != 1) {
            std::cerr << "computeTileValidityAndNormalized: invalid image" << std::endl;
            return false;
        }
        CV_Assert(grayTile.type() == CV_8U || grayTile.type() == CV_32F);

        // Work on uint8 for simplicity
        cv::Mat gray8;
        if (grayTile.type() == CV_8U) {
            gray8 = grayTile;
        } else {
            // assume [0,1] float; clamp & scale
            cv::Mat tmp;
            cv::threshold(grayTile, tmp, 1.0, 1.0, cv::THRESH_TRUNC);
            tmp.convertTo(gray8, CV_8U, 255.0);
        }

        const int W = gray8.cols;
        const int H = gray8.rows;
        const int tilesX = (W + tileSizePx - 1) / tileSizePx;
        const int tilesY = (H + tileSizePx - 1) / tileSizePx;

        outTileMask = cv::Mat1b::zeros(tilesY, tilesX);
        outNormalized = cv::Mat::zeros(H, W, CV_8U);

        auto tileRect = [&](int ty, int tx) -> cv::Rect {
            const int x0 = tx * tileSizePx;
            const int y0 = ty * tileSizePx;
            const int w = std::min(tileSizePx, W - x0);
            const int h = std::min(tileSizePx, H - y0);
            return cv::Rect(x0, y0, w, h);
        };

        for (int ty = 0; ty < tilesY; ++ty) {
            for (int tx = 0; tx < tilesX; ++tx) {
                const cv::Rect R = tileRect(ty, tx);
                cv::Mat tile = gray8(R).clone();
                bool valid = normalizeTile(tile, peakThreshold, minContrast, tileHistogramBins, validateSubtiles);
                //std::cout << "normalizeTile tx=" << tx << ", ty=" << ty << ", valid=" << valid << std::endl;
                outTileMask(ty, tx) = valid;
                tile.copyTo(outNormalized(R));
            }
        }

        return true;
    } catch (const std::exception &e) {
        std::cerr << "computeTileValidityAndNormalized exception: " << e.what() << std::endl;
        return false;
    }
}

} // namespace BitMatrixTracker
} // namespace psm
