// ESP8266 fast tensor path: tuned C for 32x32 (fixed-size loops for unrolling).

#include "pot_o/tensor_ops.h"
#include <string.h>

#define RESTRICT __restrict__

#if defined(ESP8266_DEVICE)
#define N 32
static void matmul_32(const float* RESTRICT d, float* RESTRICT out) {
    size_t i, j, k;
    for (i = 0; i < N; i++) {
        for (j = 0; j < N; j++) {
            float sum = 0.0f;
            for (k = 0; k < N; k++)
                sum += d[i * N + k] * d[k * N + j];
            out[i * N + j] = sum;
        }
    }
}

size_t tensor_execute_fast_esp8266(int op, const Tensor* RESTRICT in, float* RESTRICT out) {
    size_t out_len = in->len;
    const float* RESTRICT d = in->data;

    switch (op) {
        case OP_MATRIX_MULTIPLY:
            if (in->rows == 32 && in->cols == 32)
                matmul_32(d, out);
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
        case OP_DOT_PRODUCT:
            tensor_dot_product(d, in->len, out, &out_len);
            break;
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
