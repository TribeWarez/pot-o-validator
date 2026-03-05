#ifndef MML_SCORE_H
#define MML_SCORE_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// Byte-level entropy as proxy for compressibility (DEFLATE approximation).
// Returns out_entropy / in_entropy; lower is more compressible.
double compute_mml_score(const float* __restrict input, size_t in_len,
                         const float* __restrict output, size_t out_len);

#ifdef __cplusplus
}
#endif

#endif // MML_SCORE_H
