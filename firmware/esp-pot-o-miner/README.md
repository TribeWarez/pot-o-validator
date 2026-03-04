# ESP PoT-O Miner Firmware

[![CI](https://img.shields.io/github/actions/workflow/status/TribeWarez/esp-pot-o-miner/ci.yml?branch=main)](https://github.com/TribeWarez/esp-pot-o-miner/actions)
[![Releases](https://img.shields.io/github/v/release/TribeWarez/esp-pot-o-miner)](https://github.com/TribeWarez/esp-pot-o-miner/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Mining firmware for ESP32-S and ESP8266 microcontrollers.  
Connects to `pot.rpc.gateway.tribewarez.com` to fetch PoT-O challenges, execute tensor operations on-device, and submit proofs.

- **Releases (binaries):** [github.com/TribeWarez/esp-pot-o-miner/releases](https://github.com/TribeWarez/esp-pot-o-miner/releases)
- **Repository:** [github.com/TribeWarez/esp-pot-o-miner](https://github.com/TribeWarez/esp-pot-o-miner)
- **PoT-O gateway:** [pot.rpc.gateway.tribewarez.com](https://pot.rpc.gateway.tribewarez.com)

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
3. **Progress** (recommended): `POST /devices/progress` with `challenge_id`, `hash`, and `device_id` every few seconds while mining so the status dashboard shows live activity.
4. **Heartbeat** every 30s via `GET /health`

For full CLI and ESP integration details (curl examples, device_id on submit, progress reporting), see the validator root [CLI_AND_ESP_GUIDE.md](../../CLI_AND_ESP_GUIDE.md).

## Firmware self-check on boot (optional)

To keep devices up to date, you can perform a lightweight self-check against the status gateway when the ESP boots:

- Base URL: `https://status.rpc.gateway.tribewarez.com`
- Endpoint: `/api/device/self-check`
- Query parameters:
  - `device_type`: `esp32` or `esp8266`
  - `channel`: e.g. `testnet`
  - `kind`: `firmware`
  - `current_version`: firmware version string compiled into the binary (e.g. `POT_O_VERSION`)

Example HTTP request:

```text
GET https://status.rpc.gateway.tribewarez.com/api/device/self-check?device_type=esp32&channel=testnet&kind=firmware&current_version=1.0.0
```

If the response reports `update_required` or `update_recommended` and includes an `artifact.download_url`, you can:

1. Download the new firmware image with a small HTTPS client.
2. Verify the `checksum` (if present).
3. Flash the new image according to your OTA update strategy for the device.

The exact OTA mechanism depends on your hardware and bootloader, but this endpoint gives you a single, canonical source of truth for which firmware version to run per device type and channel.

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

## License

[MIT](LICENSE)
