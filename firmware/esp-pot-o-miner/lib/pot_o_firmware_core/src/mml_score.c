// MML score: byte-level entropy as proxy for compressibility (DEFLATE approximation)

#include "pot_o/mml_score.h"
#include <string.h>
#include <math.h>

static double entropy(const float* restrict data, size_t len, uint32_t* restrict h) {
    size_t total = len * sizeof(float);
    const uint8_t* b = (const uint8_t*)data;
    memset(h, 0, 256 * sizeof(uint32_t));
    for (size_t j = 0; j < total; j++)
        h[b[j]]++;
    double e = 0.0;
    for (int k = 0; k < 256; k++) {
        if (h[k] == 0) continue;
        double p = (double)h[k] / (double)total;
        e -= p * log(p);
    }
    return e;
}

double compute_mml_score(const float* restrict input, size_t in_len,
                         const float* restrict output, size_t out_len) {
    uint32_t hist[256];
    double in_ent = entropy(input, in_len, hist);
    double out_ent = entropy(output, out_len, hist);
    if (in_ent < 1e-9) return 1.0;
    return out_ent / in_ent;
}
