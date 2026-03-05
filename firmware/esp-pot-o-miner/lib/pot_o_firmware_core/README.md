# pot_o_firmware_core

Shared PoT-O firmware library: tensor ops, neural path, SHA-256, MML score. Usable from PlatformIO or other build systems.

## Required defines

- **Platform:** exactly one of `ESP32S_DEVICE` or `ESP8266_DEVICE`
- **Limits (optional):** `MAX_TENSOR_DIM`, `MAX_WORKING_MEM`, `NEURAL_LAYER_0/1/2`, `MAX_MINE_ITERATIONS`, `HEARTBEAT_INTERVAL_MS` (see `pot_o_config.h`)

## Optional build flags

- `USE_ESP_DSP` – enable ESP-DSP–backed tensor fast path on ESP32-S (tiled matmul, dot product)
- `USE_FAST_ACTIVATIONS` – use fast sigmoid/tanh approximations in tensor ops
- `USE_ASM_KERNELS` – enable optional Xtensa ASM kernels (matmul, Hamming) when implemented

## API

- **pot_o/tensor_ops.h** – `Tensor`, `tensor_init`, `tensor_execute`, op names and `op_from_name` / `op_supported`
- **pot_o/neural_path.h** – `expected_path`, `compute_actual_path`, `hamming_distance`, `path_to_hex`
- **pot_o/sha256_util.h** – `sha256_raw`, `SHA256Ctx` + `sha256_init` / `sha256_update` / `sha256_finish`, `bytes_to_hex`, `hex_to_bytes`. On ESP32, mbedtls is used; enable `CONFIG_MBEDTLS_HARDWARE_SHA` (e.g. in `sdkconfig.defaults`) for hardware acceleration.
- **pot_o/mml_score.h** – `compute_mml_score`
- **pot_o/pot_o_config.h** – config macros and layer widths

## Layout

- `include/pot_o/*.h` – public headers
- `src/*.c` – implementation (base + platform-specific)
- `asm/*.S` – optional Xtensa ASM (when `USE_ASM_KERNELS`)

Include path for consumers: `include` (so `#include "pot_o/tensor_ops.h"` works).
