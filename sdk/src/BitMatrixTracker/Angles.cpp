#include <vector>
#include <cmath>
#include <algorithm>
#include <numeric>
#include <iostream>

#include "Posemesh/BitMatrixTracker/Config.hpp"
#include "Posemesh/BitMatrixTracker/Types.hpp"

namespace psm {
namespace BitMatrixTracker {

static inline float angDist90(float a, float b)
{
    // distance on a circle of length 90
    float d = std::fabs(a - b);
    if (d > 90.0f)
        d = std::fmod(d, 90.0f);
    return std::min(d, 90.0f - d);
}

static inline float angDist180(float a, float b)
{
    float d = std::fabs(a - b);
    if (d > 180.0f)
        d = std::fmod(d, 180.0f);
    return std::min(d, 180.0f - d);
}

static inline float wrap90(float a)
{
    float x = std::fmod(a, 90.0f);
    if (x < 0.0f)
        x += 90.0f;
    return x;
}

static inline float wrap180(float a)
{
    float x = std::fmod(a, 180.0f);
    if (x < 0.0f)
        x += 180.0f;
    return x;
}

static float histogramPeakDeg(const std::vector<float> &angles, int bins)
{
    if (angles.empty() || bins <= 0)
        return 0.0f;
    std::vector<float> hist(bins, 0.0f);
    const float scale = bins / 90.0f;
    for (float a : angles) {
        float w = wrap90(a) * scale;
        int idx = static_cast<int>(w);
        if (idx < 0) idx = 0;
        if (idx >= bins) idx = bins - 1;
        hist[idx] += 1.0f;
    }
    // smooth with radius 2: kernel [1,2,3,2,1]
    std::vector<float> sm(bins, 0.0f);
    for (int i = 0; i < bins; ++i) {
        float acc = 0.0f, ws = 0.0f;
        for (int k = -2; k <= 2; ++k) {
            int j = i + k;
            if (j < 0) j += bins;
            if (j >= bins) j -= bins;
            float w = (k == 0 ? 3.0f : (std::abs(k) == 1 ? 2.0f : 1.0f));
            acc += w * hist[j];
            ws += w;
        }
        sm[i] = (ws > 0.0f) ? (acc / ws) : 0.0f;
    }
    int argmax = 0;
    for (int i = 1; i < bins; ++i)
        if (sm[i] > sm[argmax]) argmax = i;
    // return center angle of the bin
    const float binWidth = 90.0f / bins;
    return (argmax + 0.5f) * binWidth;
}

// grid-based clustering within 'radius' (px)
struct DSU {
    std::vector<int> p, r;
    explicit DSU(int n): p(n), r(n,0) { std::iota(p.begin(), p.end(), 0); }
    int find(int x){ return p[x]==x?x:p[x]=find(p[x]); }
    void unite(int a,int b) {
        a=find(a);
        b=find(b);
        if(a==b)
            return;
        if(r[a]<r[b])
            std::swap(a,b);
        p[b]=a;
        if(r[a]==r[b])
            r[a]++;
    }
};

static void collapseFamily(const std::vector<cv::Point2f> &pts,
                           const std::vector<float> &angs,
                           float radius,
                           std::vector<cv::Point2f> &outPts,
                           std::vector<float> &outAngles)
{
    outPts.clear();
    outAngles.clear();
    const int n = static_cast<int>(pts.size());
    if (n == 0)
        return;

    // grid hash
    const float cell = std::max(1.0f, radius);
    std::unordered_map<long long, std::vector<int>> buckets;
    buckets.reserve(n*2);

    auto keyOf = [&](int gx, int gy)->long long { return ( (static_cast<long long>(gx) << 32) ^ static_cast<unsigned long long>(gy) ); };
    auto gidx = [&](float v)->int { return static_cast<int>(std::floor(v / cell)); };

    for (int i = 0; i < n; ++i) {
        int gx = gidx(pts[i].x);
        int gy = gidx(pts[i].y);
        buckets[keyOf(gx,gy)].push_back(i);
    }

    DSU dsu(n);
    const float r2 = radius * radius;
    for (int i = 0; i < n; ++i) {
        int gx = gidx(pts[i].x);
        int gy = gidx(pts[i].y);
        for (int dy = -1; dy <= 1; ++dy) for (int dx = -1; dx <= 1; ++dx) {
            auto it = buckets.find(keyOf(gx+dx, gy+dy));
            if (it == buckets.end()) continue;
            for (int j : it->second) {
                if (j <= i) continue; // check each pair once
                float dxp = pts[i].x - pts[j].x; float dyp = pts[i].y - pts[j].y;
                if (dxp*dxp + dyp*dyp <= r2) dsu.unite(i,j);
            }
        }
    }

    // collect clusters
    std::unordered_map<int, std::vector<int>> groups;
    groups.reserve(n);
    for (int i = 0; i < n; ++i) groups[dsu.find(i)].push_back(i);

    outPts.reserve(groups.size());
    outAngles.reserve(groups.size());
    for (auto &kv : groups) {
        const auto &g = kv.second;
        // average position
        float sx=0, sy=0; for(int idx: g){
            sx += pts[idx].x;
            sy += pts[idx].y;
        }
        cv::Point2f c(sx / g.size(), sy / g.size());
        // simple average of angles (they are already near a small window), no circular wrap needed
        float sa=0;
        for(int idx: g) {
            sa += angs[idx];
        }
        float a = sa / g.size();
        outPts.push_back(c);
        outAngles.push_back(wrap180(a));
    }
}

bool groupSplitAndCollapse(const Detections &raw,
                           const Config &cfg,
                           Detections &outDiag1,
                           Detections &outDiag2,
                           int *keptLoose,
                           int *keptStrict)
{
    try {
        outDiag1.points.clear();
        outDiag1.anglesDeg.clear();
        outDiag2.points.clear();
        outDiag2.anglesDeg.clear();

        if (raw.points.empty()) {
            if (keptLoose) *keptLoose = 0;
            if (keptStrict) *keptStrict = 0;
            return true;
        }

        // 1) Global peak on all angles
        std::vector<float> allAngles = raw.anglesDeg; // copy
        const float peakLoose = histogramPeakDeg(allAngles, cfg.angleHistBins);

        // 2) Loose keep (± angleKeepDegLoose)
        std::vector<int> idxLoose; idxLoose.reserve(raw.points.size());
        for (int i = 0; i < (int)raw.points.size(); ++i) {
            float a = raw.anglesDeg[i];
            float dist = angDist90(a, peakLoose);
            if (dist <= cfg.angleKeepDegLoose)
                idxLoose.push_back(i);
        }
        if (keptLoose) *keptLoose = (int)idxLoose.size();
        if (idxLoose.empty()) {
            if (keptStrict) *keptStrict = 0;
            return true;
        }

        // 3) Recompute peak on loose survivors, then strict keep
        std::vector<float> looseAngles;
        looseAngles.reserve(idxLoose.size());
        for (int i : idxLoose) {
            looseAngles.push_back(raw.anglesDeg[i]);
        }

        float sum = 0.0f;
        for (float angle : looseAngles) {
            sum += wrap90(angle);
        }
        const float peakStrict = sum / static_cast<float>(looseAngles.size());
        std::cout << "peakStrict: " << peakStrict << std::endl;

        std::vector<cv::Point2f> pts1, pts2;
        pts1.reserve(idxLoose.size());
        pts2.reserve(idxLoose.size());
        
        std::vector<float> ang1, ang2;
        ang1.reserve(idxLoose.size());
        ang2.reserve(idxLoose.size());

        for (int i : idxLoose) {
            float a = raw.anglesDeg[i];
            float dist = angDist90(a, peakStrict);
            if (dist <= cfg.angleKeepDegStrict) {
                // Wrap around 0 .. 180 "edge"
                float dist180 = angDist180(a, peakStrict);

                // Separate into two perpendicular groups, one for each diagonal for chess markers.
                if (dist180 < 45) {
                    // First 'quadrant' near the peak (peak is always in 0-90° range)
                    pts1.push_back(raw.points[i]);
                    ang1.push_back(a);
                    //std::cout << "TYPE 1! dist180 = " << dist180 << std::endl;
                } else {
                    // Second 'quadrant', approx 90 deg away from peak
                    pts2.push_back(raw.points[i]);
                    ang2.push_back(a);
                    //std::cout << "TYPE 2! dist180 = " << dist180 << std::endl;
                }
            }
        }
        if (keptStrict) *keptStrict = (int)(pts1.size() + pts2.size());

        // 4) Collapse near-duplicates within each family
        std::vector<cv::Point2f> cpts1, cpts2; std::vector<float> cang1, cang2;
        collapseFamily(pts1, ang1, cfg.collapseRadiusPx, cpts1, cang1);
        collapseFamily(pts2, ang2, cfg.collapseRadiusPx, cpts2, cang2);

        outDiag1.points = std::move(cpts1);
        outDiag1.anglesDeg = std::move(cang1);
        outDiag2.points = std::move(cpts2);
        outDiag2.anglesDeg = std::move(cang2);

        return true;
    } catch (const std::exception &e) {
        std::cerr << "groupSplitAndCollapse exception: " << e.what() << std::endl;
        return false;
    }
}

} // namespace BitMatrixTracker
} // namespace psm
