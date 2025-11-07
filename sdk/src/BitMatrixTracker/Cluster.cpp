#include <queue>
#include <iostream>
#include <opencv2/core.hpp>
#include "Posemesh/BitMatrixTracker/Types.hpp"

namespace psm {
namespace BitMatrixTracker {

bool buildClustersFromTileMask(const cv::Mat1b &tileMask,
                               int tileSizePx,
                               int minSideLengthTiles, // Skip clusters with too small width or height (in tiles)
                               int minValidTilesCount, // Skip clusters with too few valid tiles
                               float minValidAreaRatio, // Skip clusters with a lot of invalid tiles. E.g. a long diagonal 'edge' of valid tiles.
                               float maxBoundsAspectRatio, // Skip clusters which are too far from square-ish.
                               std::vector<Cluster> &outClusters)
{
    std::vector<bool> visited(tileMask.rows * tileMask.cols, false);

    try {
        outClusters.clear();
        if (tileMask.empty())
            return true;
        const int H = tileMask.rows;
        const int W = tileMask.cols;

        const int dx[8] = { -1, 0, 1, -1, 1, -1, 0, 1 };
        const int dy[8] = { -1,-1,-1,  0, 0,  1, 1, 1 };

        for (int y0 = 0; y0 < H; ++y0) {
            for (int x0 = 0; x0 < W; ++x0) {
                if (!tileMask(y0, x0) || visited[y0 * W + x0])
                    continue;
                std::queue<cv::Point> queue; // Points to explore. Keep adding neighbors of cluster points.
                queue.emplace(x0, y0);
                visited[y0 * W + x0] = true;

                int minX = x0, maxX = x0, minY = y0, maxY = y0;
                std::vector<cv::Point> clusterPoints;
                clusterPoints.emplace_back(x0, y0);

                int validTilesInCluster = 0;
                while (!queue.empty()) {
                    cv::Point p = queue.front();
                    queue.pop();
                    for (int k = 0; k < 8; ++k) {
                        int nearbyX = p.x + dx[k];
                        int nearbyY = p.y + dy[k];
                        if (nearbyX < 0 || nearbyY < 0 || nearbyX >= W || nearbyY >= H)
                            continue;
                        if (!tileMask(nearbyY, nearbyX) || visited[nearbyY * W + nearbyX])
                            continue;
                        
                        visited[nearbyY * W + nearbyX] = true;
                        queue.emplace(nearbyX, nearbyY);
                        clusterPoints.emplace_back(nearbyX, nearbyY);
                        if (nearbyX < minX) minX = nearbyX;
                        if (nearbyX > maxX) maxX = nearbyX;
                        if (nearbyY < minY) minY = nearbyY;
                        if (nearbyY > maxY) maxY = nearbyY;
                    }
                }

                int widthTiles = maxX - minX + 1;
                int heightTiles = maxY - minY + 1;
                if (widthTiles < minSideLengthTiles || heightTiles < minSideLengthTiles) {
                    //std::cout << "Skip too small cluster (width or height < " << minSideLengthTiles << ")" << std::endl;
                    continue;
                }
                if (clusterPoints.size() < minValidTilesCount) {
                    //std::cout << "Skip too small cluster (pts.size() < " << minValidTilesCount << ")" << std::endl;
                    continue;
                }
                if (clusterPoints.size() < minValidAreaRatio * widthTiles * heightTiles) {
                    //std::cout << "Skip cluster with too low valid area ratio (ratio="
                    //          << ((float)clusterPoints.size() / widthTiles * heightTiles) << ", valid tiles: "
                    //          << clusterPoints.size() << ", area: " << (widthTiles * heightTiles)
                    //          << ", size: " << widthTiles << "x" << heightTiles << "))"
                    //          << std::endl;
                    continue;
                }
                if (widthTiles > maxBoundsAspectRatio * heightTiles || heightTiles > maxBoundsAspectRatio * widthTiles) {
                    //std::cout << "Skip cluster with too big width/height ratio ("
                    //          << widthTiles << "x" << heightTiles << ")" << std::endl;
                    continue;
                }

                
                Cluster c;
                c.tileBounds = cv::Rect(minX, minY, (maxX - minX + 1), (maxY - minY + 1));
                c.tileMask = cv::Mat1b::zeros(c.tileBounds.size());
                for (const auto &p : clusterPoints) {
                    c.tileMask(p.y - minY, p.x - minX) = 1;
                }
                const int halo = 2;
                c.pixelBounds = cv::Rect(c.tileBounds.x * tileSizePx,
                                         c.tileBounds.y * tileSizePx,
                                         c.tileBounds.width * tileSizePx,
                                         c.tileBounds.height * tileSizePx);
                c.pixelBounds.x = std::max(0, c.pixelBounds.x - halo);
                c.pixelBounds.y = std::max(0, c.pixelBounds.y - halo);

                //std::cout << "Cluster " << outClusters.size() << " tile bounds: " << c.tileBounds << ", pixel bounds: " << c.pixelBounds << std::endl;

                outClusters.emplace_back(std::move(c));
            }
        }
        return true;
    } catch (const std::exception &e) {
        std::cerr << "buildClustersFromTileMask exception: " << e.what() << std::endl;
        return false;
    }
}

} // namespace BitMatrixTracker
} // namespace psm
