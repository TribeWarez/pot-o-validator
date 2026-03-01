#ifndef TENSOR_OPS_H
#define TENSOR_OPS_H

#include <stdint.h>
#include <stddef.h>
#include <math.h>
#include <string.h>

// ── Supported operations (must match server OPERATIONS array indices) ───────
// 0=matrix_multiply 1=convolution 2=relu 3=sigmoid 4=tanh 5=dot_product 6=normalize
#define OP_MATRIX_MULTIPLY 0
#define OP_CONVOLUTION     1
#define OP_RELU            2
#define OP_SIGMOID         3
#define OP_TANH            4
#define OP_DOT_PRODUCT     5
#define OP_NORMALIZE       6

static const char* OP_NAMES[] = {
    "matrix_multiply", "convolution", "relu", "sigmoid",
    "tanh", "dot_product", "normalize"
};

static inline int op_from_name(const char* name) {
    for (int i = 0; i < 7; i++) {
        if (strcmp(name, OP_NAMES[i]) == 0) return i;
    }
    return -1;
}

#if defined(ESP8266_DEVICE)
static inline int op_supported(int op) {
    return op == OP_RELU || op == OP_SIGMOID ||
           op == OP_DOT_PRODUCT || op == OP_NORMALIZE;
}
#else
static inline int op_supported(int op) {
    return op != OP_TANH;
}
#endif

// ── Tensor data buffer ──────────────────────────────────────────────────────
typedef struct {
    float* data;
    size_t rows;
    size_t cols;
    size_t len;   // rows * cols
} Tensor;

static inline void tensor_init(Tensor* t, float* buf, size_t rows, size_t cols) {
    t->data = buf;
    t->rows = rows;
    t->cols = cols;
    t->len  = rows * cols;
}

// ── Operations ──────────────────────────────────────────────────────────────

static inline void tensor_matrix_multiply(const Tensor* in, float* out) {
    size_t n = in->rows < in->cols ? in->rows : in->cols;
    for (size_t i = 0; i < n; i++) {
        for (size_t j = 0; j < n; j++) {
            float sum = 0.0f;
            for (size_t k = 0; k < n; k++) {
                sum += in->data[i * n + k] * in->data[k * n + j];
            }
            out[i * n + j] = sum;
        }
    }
}

static inline void tensor_convolution(const float* in, size_t in_len, float* out, size_t* out_len) {
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
        for (size_t j = 0; j < klen; j++) {
            sum += in[i + j] * kernel[j];
        }
        out[i] = sum;
    }
}

static inline void tensor_relu(const float* in, float* out, size_t len) {
    for (size_t i = 0; i < len; i++) {
        out[i] = in[i] > 0.0f ? in[i] : 0.0f;
    }
}

static inline void tensor_sigmoid(const float* in, float* out, size_t len) {
    for (size_t i = 0; i < len; i++) {
        out[i] = 1.0f / (1.0f + expf(-in[i]));
    }
}

static inline void tensor_tanh_op(const float* in, float* out, size_t len) {
    for (size_t i = 0; i < len; i++) {
        out[i] = tanhf(in[i]);
    }
}

static inline void tensor_dot_product(const float* in, size_t len, float* out, size_t* out_len) {
    size_t half = len / 2;
    float dot = 0.0f;
    for (size_t i = 0; i < half; i++) {
        dot += in[i] * in[half + i];
    }
    out[0] = dot;
    *out_len = 1;
}

static inline void tensor_normalize(const float* in, float* out, size_t len) {
    float mag = 0.0f;
    for (size_t i = 0; i < len; i++) mag += in[i] * in[i];
    mag = sqrtf(mag);
    if (mag > 1e-7f) {
        for (size_t i = 0; i < len; i++) out[i] = in[i] / mag;
    } else {
        memcpy(out, in, len * sizeof(float));
    }
}

// Dispatch: execute operation `op` on input, writing result to out.
// Returns output length. out buffer must be at least in->len elements.
static inline size_t tensor_execute(int op, const Tensor* in, float* out) {
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

#endif // TENSOR_OPS_H
