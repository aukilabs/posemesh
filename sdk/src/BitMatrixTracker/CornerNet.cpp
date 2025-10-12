#include "Posemesh/BitMatrixTracker/CornerNet.hpp"

#include <cmath>
#include <iostream>
#include <vector>
#include <cstdio>
#include <cstring>


namespace psm {
namespace BitMatrixTracker {

static inline float relu(float x) { return x > 0.0f ? x : 0.0f; }
static inline float sigmoid(float x) { return 1.0f / (1.0f + std::exp(-x)); }
static inline float tanh01(float x) { return std::tanh(x); }

bool CornerNetWeights::isValid() const
{
    return W1.size() == 32u * 25u && b1.size() == 32u &&
           W2.size() == 24u * 32u && b2.size() == 24u &&
           Wc.size() == 24u && Wr.size() == 24u;
}

// Minimalistic binary format (little-endian floats):
// magic(8) = "CRNRNET\0", version(uint32)=1, then arrays in order:
// W1(32*25), b1(32), W2(24*32), b2(24), Wc(24), bc(1), Wr(24), br(1)
bool loadCornerNetWeightsFromFile(const std::string &path, CornerNetWeights &out)
{
    FILE *f = std::fopen(path.c_str(), "rb");
    if (!f) {
        std::cerr << "CornerNet: could not open weights file: " << path << std::endl;
        return false;
    }
    char magic[8] = {0};
    if (std::fread(magic, 1, 8, f) != 8 || std::memcmp(magic, "CRNRNET\0", 8) != 0) {
        std::cerr << "CornerNet: bad magic" << std::endl;
        std::fclose(f);
        return false;
    }
    uint32_t ver = 0;
    if (std::fread(&ver, sizeof(uint32_t), 1, f) != 1 || ver != 1) {
        std::cerr << "CornerNet: bad version" << std::endl;
        std::fclose(f);
        return false;
    }
    auto readVec = [&](std::vector<float> &v, size_t n) {
        v.resize(n);
        return std::fread(v.data(), sizeof(float), n, f) == n;
    };
    if (!readVec(out.W1, 32u * 25u)) goto fail;
    if (!readVec(out.b1, 32u)) goto fail;
    if (!readVec(out.W2, 24u * 32u)) goto fail;
    if (!readVec(out.b2, 24u)) goto fail;
    if (!readVec(out.Wc, 24u)) goto fail;
    if (std::fread(&out.bc, sizeof(float), 1, f) != 1) goto fail;
    if (!readVec(out.Wr, 24u)) goto fail;
    if (std::fread(&out.br, sizeof(float), 1, f) != 1) goto fail;

    std::fclose(f);
    if (!out.isValid()) {
        std::cerr << "CornerNet: invalid sizes after load" << std::endl;
        return false;
    }
    return true;
fail:
    std::cerr << "CornerNet: truncated weights file" << std::endl;
    std::fclose(f);
    return false;
}

static inline void matvec_rowmajor(const float *W, const float *x, float *y,
                                   int rows, int cols)
{
    for (int r = 0; r < rows; ++r) {
        const float *wr = W + r * cols;
        float acc = 0.0f;
        for (int c = 0; c < cols; ++c)
            acc += wr[c] * x[c];
        y[r] = acc;
    }
}

static inline void add_bias_relu(float *y, const float *b, int n)
{
    for (int i = 0; i < n; ++i) y[i] = relu(y[i] + b[i]);
}

static inline void add_bias(float *y, const float *b, int n)
{
    for (int i = 0; i < n; ++i) y[i] = y[i] + b[i];
}

// Forward for a single 5x5 vector x[25] in [0,1].
static inline void forward25(const float *x, const CornerNetWeights &w, float &conf, float &rot)
{
    float h1[32];
    float h2[24];

    matvec_rowmajor(w.W1.data(), x, h1, 32, 25);
    add_bias_relu(h1, w.b1.data(), 32);

    matvec_rowmajor(w.W2.data(), h1, h2, 24, 32);
    add_bias_relu(h2, w.b2.data(), 24);

    // Heads
    float confLogit = 0.0f, rotT = 0.0f;
    for (int i = 0; i < 24; ++i) {
        confLogit += w.Wc[i] * h2[i];
        rotT += w.Wr[i] * h2[i];
    }
    confLogit += w.bc;
    rotT += w.br;

    conf = sigmoid(confLogit);
    rot = tanh01(rotT); // in [-1, 1]
}

void runCornerNet5x5F32(const float *patch5x5, const CornerNetWeights &w, float &conf, float &angleDeg)
{
    float rot;
    forward25(patch5x5, w, conf, rot);
    // Map to [0, 90)
    float angle = rot * 90.0f; // rot in [-1,1]
    // wrap to [0,90)
    angle = std::fmod(std::fmod(angle, 90.0f) + 90.0f, 90.0f);
    angleDeg = angle;
}

void runCornerNet5x5U8(const uint8_t *patch5x5, const CornerNetWeights &w, float &conf, float &angleDeg)
{
    float x[25];
    for (int i = 0; i < 25; ++i) x[i] = static_cast<float>(patch5x5[i]) / 255.0f;
    runCornerNet5x5F32(x, w, conf, angleDeg);
}

} // namespace BitMatrixTracker
} // namespace psm
