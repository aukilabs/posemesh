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
                               std::vector<Cluster> &outClusters)
{
    try {
        outClusters.clear();
        if (tileMask.empty())
            return true;
        const int H = tileMask.rows;
        const int W = tileMask.cols;

        cv::Mat1b visited = cv::Mat1b::zeros(H, W);
        auto inb = [&](int y, int x) { return (unsigned)x < (unsigned)W && (unsigned)y < (unsigned)H; };
        const int dx[8] = { -1, 0, 1, -1, 1, -1, 0, 1 };
        const int dy[8] = { -1,-1,-1,  0, 0,  1, 1, 1 };

        for (int y0 = 0; y0 < H; ++y0) {
            for (int x0 = 0; x0 < W; ++x0) {
                if (!tileMask(y0, x0) || visited(y0, x0))
                    continue;
                std::queue<cv::Point> q;
                q.emplace(x0, y0);
                visited(y0, x0) = 1;

                int minx = x0, maxx = x0, miny = y0, maxy = y0;
                std::vector<cv::Point> pts;
                pts.emplace_back(x0, y0);

                int validTilesInCluster = 0;
                while (!q.empty()) {
                    cv::Point p = q.front(); q.pop();
                    for (int k = 0; k < 8; ++k) {
                        int nx = p.x + dx[k];
                        int ny = p.y + dy[k];
                        if (!inb(ny, nx))
                            continue;
                        if (!tileMask(ny, nx) || visited(ny, nx))
                            continue;
                        visited(ny, nx) = 1;
                        q.emplace(nx, ny);
                        pts.emplace_back(nx, ny);
                        if (nx < minx) minx = nx;
                        if (nx > maxx) maxx = nx;
                        if (ny < miny) miny = ny;
                        if (ny > maxy) maxy = ny;
                    }
                }

                int widthTiles = maxx - minx + 1;
                int heightTiles = maxy - miny + 1;
                if (widthTiles < minSideLengthTiles || heightTiles < minSideLengthTiles) {
                    //std::cout << "Skip too small cluster (width or height < " << minSideLengthTiles << ")" << std::endl;
                    continue;
                }
                if (pts.size() < minValidTilesCount) {
                    //std::cout << "Skip too small cluster (pts.size() < " << minValidTilesCount << ")" << std::endl;
                    continue;
                }
                
                Cluster c;
                c.tileBounds = cv::Rect(minx, miny, (maxx - minx + 1), (maxy - miny + 1));
                c.tileMask = cv::Mat1b::zeros(c.tileBounds.size());
                for (const auto &p : pts) {
                    c.tileMask(p.y - miny, p.x - minx) = 1;
                }
                const int halo = 2;
                c.pixelBounds = cv::Rect(c.tileBounds.x * tileSizePx,
                                         c.tileBounds.y * tileSizePx,
                                         c.tileBounds.width * tileSizePx,
                                         c.tileBounds.height * tileSizePx);
                c.pixelBounds.x = std::max(0, c.pixelBounds.x - halo);
                c.pixelBounds.y = std::max(0, c.pixelBounds.y - halo);

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
