// Base (portable) tensor op implementations with restrict and branchless ReLU

#include "pot_o/tensor_ops.h"
#include <math.h>
#include <string.h>

#define RESTRICT __restrict__

void tensor_matrix_multiply(const Tensor* RESTRICT in, float* RESTRICT out) {
    size_t n = in->rows < in->cols ? in->rows : in->cols;
    const float* RESTRICT d = in->data;
    for (size_t i = 0; i < n; i++) {
        for (size_t j = 0; j < n; j++) {
            float sum = 0.0f;
            for (size_t k = 0; k < n; k++)
                sum += d[i * n + k] * d[k * n + j];
            out[i * n + j] = sum;
        }
    }
}

void tensor_convolution(const float* RESTRICT in, size_t in_len, float* RESTRICT out, size_t* RESTRICT out_len) {
    static const float kernel[] = {0.25f, 0.5f, 0.25f};
    const size_t klen = 3;
    if (in_len < klen) {
        memcpy(out, in, in_len * sizeof(float));
        *out_len = in_len;
        return;
    }
    *out_len = in_len - klen + 1;
    for (size_t i = 0; i < *out_len; i++) {
        float sum = 0.0f;
        for (size_t j = 0; j < klen; j++)
            sum += in[i + j] * kernel[j];
        out[i] = sum;
    }
}

void tensor_relu(const float* RESTRICT in, float* RESTRICT out, size_t len) {
    for (size_t i = 0; i < len; i++)
        out[i] = in[i] * (float)(in[i] > 0.0f);
}

#if defined(USE_FAST_ACTIVATIONS)
/* Fast approximations without expf/tanhf in hot path */
static inline float fast_sigmoid(float x) {
    if (x >= 4.0f) return 1.0f;
    if (x <= -4.0f) return 0.0f;
    return 0.5f + 0.5f * (x / (1.0f + (x < 0.0f ? -x : x)));
}
static inline float fast_tanh(float x) {
    if (x >= 3.0f) return 1.0f;
    if (x <= -3.0f) return -1.0f;
    return x / (1.0f + (x < 0.0f ? -x : x));
}
#endif

void tensor_sigmoid(const float* RESTRICT in, float* RESTRICT out, size_t len) {
#if defined(USE_FAST_ACTIVATIONS)
    for (size_t i = 0; i < len; i++)
        out[i] = fast_sigmoid(in[i]);
#else
    for (size_t i = 0; i < len; i++)
        out[i] = 1.0f / (1.0f + expf(-in[i]));
#endif
}

void tensor_tanh_op(const float* RESTRICT in, float* RESTRICT out, size_t len) {
#if defined(USE_FAST_ACTIVATIONS)
    for (size_t i = 0; i < len; i++)
        out[i] = fast_tanh(in[i]);
#else
    for (size_t i = 0; i < len; i++)
        out[i] = tanhf(in[i]);
#endif
}

void tensor_dot_product(const float* RESTRICT in, size_t len, float* RESTRICT out, size_t* RESTRICT out_len) {
    size_t half = len / 2;
    float dot = 0.0f;
    for (size_t i = 0; i < half; i++)
        dot += in[i] * in[half + i];
    out[0] = dot;
    *out_len = 1;
}

void tensor_normalize(const float* RESTRICT in, float* RESTRICT out, size_t len) {
    float mag = 0.0f;
    for (size_t i = 0; i < len; i++)
        mag += in[i] * in[i];
    mag = sqrtf(mag);
    if (mag > 1e-7f) {
        for (size_t i = 0; i < len; i++)
            out[i] = in[i] / mag;
    } else {
        memcpy(out, in, len * sizeof(float));
    }
}

static size_t tensor_execute_base(int op, const Tensor* RESTRICT in, float* RESTRICT out) {
    size_t out_len = in->len;
    switch (op) {
        case OP_MATRIX_MULTIPLY:
            tensor_matrix_multiply(in, out);
            break;
        case OP_CONVOLUTION:
            tensor_convolution(in->data, in->len, out, &out_len);
            break;
        case OP_RELU:
            tensor_relu(in->data, out, in->len);
            break;
        case OP_SIGMOID:
            tensor_sigmoid(in->data, out, in->len);
            break;
        case OP_TANH:
            tensor_tanh_op(in->data, out, in->len);
            break;
        case OP_DOT_PRODUCT:
            tensor_dot_product(in->data, in->len, out, &out_len);
            break;
        case OP_NORMALIZE:
            tensor_normalize(in->data, out, in->len);
            break;
        default:
            tensor_relu(in->data, out, in->len);
            break;
    }
    return out_len;
}

#if defined(ESP32S_DEVICE)
extern size_t tensor_execute_fast_esp32(int op, const Tensor* restrict in, float* restrict out);
#endif
#if defined(ESP8266_DEVICE)
extern size_t tensor_execute_fast_esp8266(int op, const Tensor* restrict in, float* restrict out);
#endif

size_t tensor_execute(int op, const Tensor* RESTRICT in, float* RESTRICT out) {
#if defined(ESP32S_DEVICE)
    if (in->rows == in->cols && (in->rows == 64 || in->rows == 32))
        return tensor_execute_fast_esp32(op, in, out);
#endif
#if defined(ESP8266_DEVICE)
    if (in->rows == in->cols && in->rows == 32)
        return tensor_execute_fast_esp8266(op, in, out);
#endif
    return tensor_execute_base(op, in, out);
}
