#ifndef TENSOR_OPS_H
#define TENSOR_OPS_H

#include <stdint.h>
#include <stddef.h>
#include <string.h>

#ifdef __cplusplus
extern "C" {
#endif

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

// ── Public API (implementations in tensor_ops_base.c / tensor_ops_fast_*.c) ──
// Restrict qualifiers for compiler optimization; buffers must not alias.

void tensor_matrix_multiply(const Tensor* __restrict in, float* __restrict out);
void tensor_convolution(const float* __restrict in, size_t in_len, float* __restrict out, size_t* __restrict out_len);
void tensor_relu(const float* __restrict in, float* __restrict out, size_t len);
void tensor_sigmoid(const float* __restrict in, float* __restrict out, size_t len);
void tensor_tanh_op(const float* __restrict in, float* __restrict out, size_t len);
void tensor_dot_product(const float* __restrict in, size_t len, float* __restrict out, size_t* __restrict out_len);
void tensor_normalize(const float* __restrict in, float* __restrict out, size_t len);

// Dispatch: execute operation `op` on input, writing result to out.
// Returns output length. out buffer must be at least in->len elements.
// Dispatches to fast path when USE_ESP_DSP/USE_ASM_KERNELS and dimensions match.
size_t tensor_execute(int op, const Tensor* __restrict in, float* __restrict out);

#ifdef __cplusplus
}
#endif

#endif // TENSOR_OPS_H
