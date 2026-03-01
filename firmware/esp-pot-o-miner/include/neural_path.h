#ifndef NEURAL_PATH_H
#define NEURAL_PATH_H

#include <stdint.h>
#include <stddef.h>
#include <math.h>
#include <string.h>
#include "sha256_util.h"
#include "pot_o_config.h"

// Layer widths must match the server: [32, 16, 8]
static const size_t LAYER_WIDTHS[NEURAL_LAYERS] = {
    NEURAL_LAYER_0, NEURAL_LAYER_1, NEURAL_LAYER_2
};

// Compute expected path signature from challenge_id (hex string).
// Mirrors neural_path.rs::expected_path_signature
static inline void expected_path(const char* challenge_id_hex,
                                 uint8_t* path_out,
                                 size_t* path_len) {
    uint8_t hash_bytes[32];
    hex_to_bytes(challenge_id_hex, hash_bytes, 32);

    uint8_t seed[32];
    sha256_raw(hash_bytes, 32, seed);

    size_t idx = 0;
    for (int layer = 0; layer < NEURAL_LAYERS; layer++) {
        for (size_t i = 0; i < LAYER_WIDTHS[layer]; i++) {
            size_t byte_idx = i % 32;
            uint8_t bit = (seed[byte_idx] >> (i % 8)) & 1;
            path_out[idx++] = bit;
        }
        // Re-hash seed for next layer
        uint8_t next_seed[32];
        sha256_raw(seed, 32, next_seed);
        memcpy(seed, next_seed, 32);
    }
    *path_len = idx;
}

// Compute actual path from tensor output data with a given nonce.
// Mirrors neural_path.rs::compute_actual_path
static inline void compute_actual_path(const float* tensor_data,
                                       size_t tensor_len,
                                       uint64_t nonce,
                                       uint8_t* path_out,
                                       size_t* path_len) {
    // Mix nonce into a working copy (stack-allocated up to a reasonable limit)
    float mixed[MAX_TENSOR_DIM * MAX_TENSOR_DIM];
    size_t work_len = tensor_len < (MAX_TENSOR_DIM * MAX_TENSOR_DIM)
                    ? tensor_len
                    : (MAX_TENSOR_DIM * MAX_TENSOR_DIM);

    for (size_t i = 0; i < work_len; i++) {
        float nonce_contrib = sinf((float)(nonce + i) * 1e-6f) * 0.1f;
        mixed[i] = tensor_data[i] + nonce_contrib;
    }

    float* activations = mixed;
    size_t act_len = work_len;
    size_t idx = 0;

    // Temporary layer output buffer (max layer width is 32)
    float layer_out[32];

    for (int layer = 0; layer < NEURAL_LAYERS; layer++) {
        size_t width = LAYER_WIDTHS[layer];
        size_t stride = act_len / width;
        if (stride < 1) stride = 1;

        for (size_t j = 0; j < width; j++) {
            size_t start = j * stride;
            size_t end = start + stride;
            if (end > act_len) end = act_len;
            float sum = 0.0f;
            for (size_t k = start; k < end; k++) {
                sum += activations[k];
            }
            // ReLU
            layer_out[j] = sum > 0.0f ? sum : 0.0f;
            path_out[idx++] = layer_out[j] > 0.0f ? 1 : 0;
        }

        // Feed forward: layer output becomes next input
        memcpy(mixed, layer_out, width * sizeof(float));
        activations = mixed;
        act_len = width;
    }
    *path_len = idx;
}

static inline uint32_t hamming_distance(const uint8_t* a, const uint8_t* b, size_t len) {
    uint32_t dist = 0;
    for (size_t i = 0; i < len; i++) {
        if (a[i] != b[i]) dist++;
    }
    return dist;
}

// Encode path bits to hex (same as server)
static inline void path_to_hex(const uint8_t* path, size_t path_len,
                                char* hex_out) {
    size_t byte_count = (path_len + 7) / 8;
    for (size_t i = 0; i < byte_count; i++) {
        uint8_t byte_val = 0;
        for (size_t b = 0; b < 8 && (i * 8 + b) < path_len; b++) {
            if (path[i * 8 + b] != 0) {
                byte_val |= (1 << b);
            }
        }
        sprintf(hex_out + i * 2, "%02x", byte_val);
    }
    hex_out[byte_count * 2] = '\0';
}

#endif // NEURAL_PATH_H
