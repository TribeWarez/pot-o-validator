#ifndef POT_O_CONFIG_H
#define POT_O_CONFIG_H

// ── WiFi ────────────────────────────────────────────────────────────────────
// Set via build flags or override here
#ifndef WIFI_SSID
#define WIFI_SSID "YOUR_SSID"
#endif
#ifndef WIFI_PASS
#define WIFI_PASS "YOUR_PASS"
#endif

// ── RPC endpoint ────────────────────────────────────────────────────────────
#ifndef POT_RPC_HOST
#define POT_RPC_HOST "pot.rpc.gateway.tribewarez.com"
#endif
#ifndef POT_RPC_PORT
#define POT_RPC_PORT 443
#endif
#ifndef POT_RPC_TLS
#define POT_RPC_TLS 1
#endif

// ── Device constraints ──────────────────────────────────────────────────────
#ifndef MAX_TENSOR_DIM
  #if defined(ESP32S_DEVICE)
    #define MAX_TENSOR_DIM 64
  #else
    #define MAX_TENSOR_DIM 32
  #endif
#endif

#ifndef MAX_WORKING_MEM
  #if defined(ESP32S_DEVICE)
    #define MAX_WORKING_MEM (320 * 1024)
  #else
    #define MAX_WORKING_MEM (80 * 1024)
  #endif
#endif

// ── Mining ───────────────────────────────────────────────────────────────────
#ifndef MAX_MINE_ITERATIONS
#define MAX_MINE_ITERATIONS 10000
#endif
#ifndef HEARTBEAT_INTERVAL_MS
#define HEARTBEAT_INTERVAL_MS 30000
#endif

// Neural path layer widths (must match server: 32, 16, 8 = 56 neurons)
#define NEURAL_LAYERS       3
#define NEURAL_LAYER_0      32
#define NEURAL_LAYER_1      16
#define NEURAL_LAYER_2      8
#define NEURAL_TOTAL_NEURONS (NEURAL_LAYER_0 + NEURAL_LAYER_1 + NEURAL_LAYER_2)

#endif // POT_O_CONFIG_H
