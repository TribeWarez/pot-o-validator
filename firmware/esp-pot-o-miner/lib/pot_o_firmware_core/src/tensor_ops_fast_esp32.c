// ESP32-S fast tensor path: tiled matmul (8x8), optional ESP-DSP for dot product

#include "pot_o/tensor_ops.h"
#include <string.h>

#if defined(USE_ESP_DSP)
#include "esp_dsp.h"
#endif

#define TILE 8
#define RESTRICT __restrict__

#if defined(ESP32S_DEVICE)

static void matmul_tiled(const float* RESTRICT d, float* RESTRICT out, size_t n) {
    memset(out, 0, n * n * sizeof(float));
    for (size_t ii = 0; ii < n; ii += TILE) {
        for (size_t jj = 0; jj < n; jj += TILE) {
            for (size_t kk = 0; kk < n; kk += TILE) {
                size_t i_end = ii + TILE <= n ? ii + TILE : n;
                size_t j_end = jj + TILE <= n ? jj + TILE : n;
                size_t k_end = kk + TILE <= n ? kk + TILE : n;
                for (size_t i = ii; i < i_end; i++) {
                    for (size_t j = jj; j < j_end; j++) {
                        float sum = out[i * n + j];
                        for (size_t k = kk; k < k_end; k++)
                            sum += d[i * n + k] * d[k * n + j];
                        out[i * n + j] = sum;
                    }
                }
            }
        }
    }
}

size_t tensor_execute_fast_esp32(int op, const Tensor* RESTRICT in, float* RESTRICT out) {
    size_t out_len = in->len;
    size_t n = in->rows <= in->cols ? in->rows : in->cols;
    const float* RESTRICT d = in->data;

    switch (op) {
        case OP_MATRIX_MULTIPLY:
            if (in->rows == in->cols && (n == 64 || n == 32))
                matmul_tiled(d, out, n);
            else
                tensor_matrix_multiply(in, out);
            break;
        case OP_CONVOLUTION:
            tensor_convolution(d, in->len, out, &out_len);
            break;
        case OP_RELU:
            tensor_relu(d, out, in->len);
            break;
        case OP_SIGMOID:
            tensor_sigmoid(d, out, in->len);
            break;
        case OP_TANH:
            tensor_tanh_op(d, out, in->len);
            break;
        case OP_DOT_PRODUCT: {
#if defined(USE_ESP_DSP)
            size_t half = in->len / 2;
            float dot = 0.0f;
            dsps_dotprod_f32((float*)&d[0], (float*)&d[half], &dot, (int)half);
            out[0] = dot;
            out_len = 1;
#else
            tensor_dot_product(d, in->len, out, &out_len);
#endif
            break;
        }
        case OP_NORMALIZE:
            tensor_normalize(d, out, in->len);
            break;
        default:
            tensor_relu(d, out, in->len);
            break;
    }
    return out_len;
}

#endif
