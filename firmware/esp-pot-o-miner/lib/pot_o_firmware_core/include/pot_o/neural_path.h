#ifndef NEURAL_PATH_H
#define NEURAL_PATH_H

#include <stdint.h>
#include <stddef.h>
#include "pot_o_config.h"

#ifdef __cplusplus
extern "C" {
#endif

// Layer widths must match the server: [32, 16, 8]
static const size_t LAYER_WIDTHS[NEURAL_LAYERS] = {
    NEURAL_LAYER_0, NEURAL_LAYER_1, NEURAL_LAYER_2
};

// Compute expected path signature from challenge_id (hex string).
// Mirrors neural_path.rs::expected_path_signature
void expected_path(const char* challenge_id_hex,
                   uint8_t* path_out,
                   size_t* path_len);

// Compute actual path from tensor output data with a given nonce.
// Mirrors neural_path.rs::compute_actual_path
void compute_actual_path(const float* __restrict tensor_data,
                         size_t tensor_len,
                         uint64_t nonce,
                         uint8_t* __restrict path_out,
                         size_t* path_len);

uint32_t hamming_distance(const uint8_t* __restrict a, const uint8_t* __restrict b, size_t len);

// Encode path bits to hex (same as server)
void path_to_hex(const uint8_t* path, size_t path_len, char* hex_out);

#ifdef __cplusplus
}
#endif

#endif // NEURAL_PATH_H
