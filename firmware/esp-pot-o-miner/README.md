# ESP PoT-O Miner Firmware

Mining firmware for ESP32-S and ESP8266 microcontrollers.  
Connects to `pot.rpc.gateway.tribewarez.com` to fetch PoT-O challenges, execute tensor operations on-device, and submit proofs.

## Device Capabilities

| Device   | Max Tensor | Working Memory | Supported Ops |
|----------|-----------|----------------|---------------|
| ESP32-S  | 64×64     | 320 KB         | matrix_multiply, convolution, relu, sigmoid, dot_product, normalize |
| ESP8266  | 32×32     | 80 KB          | relu, sigmoid, dot_product, normalize |

## Quick Start

### Prerequisites

- [PlatformIO](https://platformio.org/install/cli)
- USB cable connected to ESP device

### Configure WiFi

Edit `include/pot_o_config.h`:

```c
#define WIFI_SSID "YOUR_SSID"
#define WIFI_PASS "YOUR_PASS"
```

Or pass via build flags:

```bash
pio run -e esp32s -t upload --build-flag="-DWIFI_SSID='\"MySSID\"' -DWIFI_PASS='\"MyPass\"'"
```

### Build & Upload

```bash
# ESP32-S
pio run -e esp32s -t upload

# ESP8266 (NodeMCU v2)
pio run -e esp8266 -t upload

# Monitor serial output
pio device monitor -b 115200
```

## Mining Flow

1. **Boot** → Connect WiFi → Register device with RPC
2. **Loop**:
   - `POST /challenge` → get tensor challenge
   - Execute tensor operation (matrix multiply, convolution, activation, etc.)
   - Compute MML score (entropy-based approximation of DEFLATE ratio)
   - Search nonces for neural path match within Hamming distance tolerance
   - `POST /submit` → submit proof for on-chain verification
3. **Heartbeat** every 30s via `GET /health`

## Architecture

```
include/
  pot_o_config.h    - Device constraints, WiFi, RPC endpoint config
  tensor_ops.h      - All tensor operations (matrix multiply, conv, activations)
  neural_path.h     - Neural path computation and validation
  sha256_util.h     - SHA-256 for ESP32 (mbedtls) and ESP8266 (BearSSL)
src/
  main.cpp          - Setup, mining loop, HTTP client, proof assembly
```

## RPC Endpoint

Default: `https://pot.rpc.gateway.tribewarez.com`

Override in `platformio.ini` build flags:

```ini
build_flags =
    -DPOT_RPC_HOST=\"your-host.example.com\"
    -DPOT_RPC_PORT=8900
    -DPOT_RPC_TLS=0
```
