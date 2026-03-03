# CLI and ESP guide (PoT-O validator)

How to register devices, report progress (current calculation + hash), and submit proofs from **CLI** (curl / scripts) and **ESP** (ESP32 / ESP8266 firmware).

**Base URL:** Your validator, e.g. `https://pot.rpc.gateway.tribewarez.com` or `http://localhost:8900`.

---

## 1. CLI guide

### 1.1 Register a device (optional but recommended)

Use a stable `device_id` (e.g. hostname or MAC) so one machine shows as one device even with multiple threads or pubkeys.

```bash
VALIDATOR="https://pot.rpc.gateway.tribewarez.com"

curl -s -X POST "$VALIDATOR/devices/register" \
  -H "Content-Type: application/json" \
  -d '{
    "device_type": "cpu",
    "device_id": "my-laptop-001",
    "miner_pubkey": "YOUR_SOLANA_PUBKEY_BASE58"
  }'
```

Response: `{ "registered": true, "device_type": "cpu", "device_id": "my-laptop-001", "miner_registered": true }`.  
Save `device_id` for progress and submit (or omit and use `miner_pubkey` + `device_type` only).

**Device types:** `esp32`, `esp8266`, `gpu`, `cpu`, `native`, `wasm`.

### 1.2 Get a challenge

```bash
curl -s -X POST "$VALIDATOR/challenge" \
  -H "Content-Type: application/json" \
  -d '{"device_type": "cpu"}'
```

Response includes `id`, `slot`, `operation_type`, `difficulty`, tensors, etc. Use `id` as `challenge_id` for progress and proof.

### 1.3 Report current calculation (progress) in real time

While your miner is working on a challenge, send the **current running calculation with hash** every few seconds so the dashboard shows live activity.

```bash
CHALLENGE_ID="challenge-uuid-from-step-1.2"
HASH="hex-sha256-of-current-state-or-work-in-progress"
DEVICE_ID="my-laptop-001"

curl -s -X POST "$VALIDATOR/devices/progress" \
  -H "Content-Type: application/json" \
  -d "{
    \"device_id\": \"$DEVICE_ID\",
    \"challenge_id\": \"$CHALLENGE_ID\",
    \"hash\": \"$HASH\"
  }"
```

Response: `{ "ok": true, "updated": true }`.

**Without `device_id`** (use miner pubkey + device type):

```bash
curl -s -X POST "$VALIDATOR/devices/progress" \
  -H "Content-Type: application/json" \
  -d "{
    \"miner_pubkey\": \"YOUR_SOLANA_PUBKEY_BASE58\",
    \"device_type\": \"cpu\",
    \"challenge_id\": \"$CHALLENGE_ID\",
    \"hash\": \"$HASH\"
  }"
```

**What to use as `hash`:** Any deterministic hash of the current work (e.g. SHA-256 of current tensor state, or of `challenge_id + nonce_range`, or intermediate proof state). The validator only stores and displays it; it does not verify the value.

### 1.4 Submit proof

After solving the challenge, POST the proof. Include `device_id` (and optionally `device_type`) so the registry stays one entry per device.

```bash
# Proof object from your miner (structure defined by PoT-O consensus)
curl -s -X POST "$VALIDATOR/submit" \
  -H "Content-Type: application/json" \
  -d "{
    \"proof\": { ... },
    \"device_id\": \"$DEVICE_ID\",
    \"device_type\": \"cpu\"
  }"
```

Response: `{ "accepted": true, "tx_signature": "..." }` or `{ "accepted": false, "error": "..." }`.

### 1.5 Full CLI loop (example)

```bash
VALIDATOR="https://pot.rpc.gateway.tribewarez.com"
DEVICE_ID="cli-miner-001"

# 1) Register once
curl -s -X POST "$VALIDATOR/devices/register" \
  -H "Content-Type: application/json" \
  -d "{\"device_type\":\"cpu\",\"device_id\":\"$DEVICE_ID\"}"

# 2) Get challenge
CHALLENGE=$(curl -s -X POST "$VALIDATOR/challenge" \
  -H "Content-Type: application/json" \
  -d '{"device_type":"cpu"}')
CHALLENGE_ID=$(echo "$CHALLENGE" | jq -r '.id')

# 3) While mining: periodically report progress (e.g. every 2–5 s)
#    HASH = your implementation (e.g. sha256 of current tensor or nonce)
# send_progress() {
#   local hash=$(echo -n "$CURRENT_STATE" | sha256sum | cut -c1-64)
#   curl -s -X POST "$VALIDATOR/devices/progress" \
#     -H "Content-Type: application/json" \
#     -d "{\"device_id\":\"$DEVICE_ID\",\"challenge_id\":\"$CHALLENGE_ID\",\"hash\":\"$hash\"}"
# }

# 4) Build proof (your miner logic), then submit
# curl -s -X POST "$VALIDATOR/submit" -H "Content-Type: application/json" \
#   -d "{\"proof\":$(cat proof.json),\"device_id\":\"$DEVICE_ID\",\"device_type\":\"cpu\"}"
```

### 1.6 Self-check for updates (CLI / CPU miners)

Before starting mining, CLI-based miners can self-check against the central gateway to see if an updated binary is available.

```bash
STATUS_API="https://status.rpc.gateway.tribewarez.com"
DEVICE_TYPE="cli"
CHANNEL="testnet"
CURRENT_VERSION="0.1.0" # your CLI binary version (semver)

curl -s "$STATUS_API/api/device/self-check" \
  -G \
  --data-urlencode "device_type=$DEVICE_TYPE" \
  --data-urlencode "channel=$CHANNEL" \
  --data-urlencode "kind=cli" \
  --data-urlencode "current_version=$CURRENT_VERSION"
```

Example JSON response:

```json
{
  "service": "pot-o-device-update",
  "device_type": "cli",
  "channel": "testnet",
  "kind": "cli",
  "current_version": "0.1.0",
  "latest_version": "0.2.0",
  "min_supported_version": "0.1.0",
  "up_to_date": false,
  "update_required": false,
  "update_recommended": true,
  "artifact": {
    "version": "0.2.0",
    "download_url": "https://downloads.gateway.tribewarez.com/cli/testnet/pot-o-validator-cli-0.2.0.tar.gz",
    "checksum": "sha256:...",
    "size_bytes": 123456
  },
  "generated_at": "2025-01-01T00:00:00.000Z"
}
```

Typical flow:

- If `update_required` is true, refuse to start and instruct the operator to update.
- If `update_recommended` is true, log a warning and optionally auto-download from `artifact.download_url`.
- If `up_to_date` is true, continue as normal.

---

## 2. ESP guide (ESP32 / ESP8266)

The firmware under `firmware/esp-pot-o-miner/` already does:

1. **Boot** → WiFi → **`POST /devices/register`** with `device_type` (`esp32s` / `esp8266`) and `device_id` = **MAC address** (`WiFi.macAddress()`).
2. **Loop:** **`POST /challenge`** → run tensor op + MML + nonce search → **`POST /submit`** with proof.

To align with the “current running calculation with hash in real time” requirement and keep dashboard stats correct, add the following.

### 2.1 Use `device_id` and `device_type` on submit

In `submit_proof()`, include the same `device_id` (and `device_type`) you used at registration so the validator counts one device per ESP. Example change in `firmware/esp-pot-o-miner/src/main.cpp`:

```cpp
bool submit_proof(const JsonDocument& proof) {
    JsonDocument submit_doc;
    submit_doc["proof"] = proof;
    submit_doc["device_id"] = g_device_id;   // from register_device()
#if defined(ESP32S_DEVICE)
    submit_doc["device_type"] = "esp32s";
#else
    submit_doc["device_type"] = "esp8266";
#endif
    String body;
    serializeJson(submit_doc, body);
    // ... rpc_post("/submit", body, resp) ...
}
```

### 2.2 Report progress during mining: `POST /devices/progress`

Call **`POST /devices/progress`** periodically while the device is working on a challenge (e.g. every 2–5 seconds, or after each nonce batch). Payload: `challenge_id`, `hash`, and `device_id` (same as registration).

**Suggested place:** inside the mining loop in `mine_challenge()`, e.g. after each N nonces or on a timer. Keep the body small to save bandwidth.

**Example – report progress once per batch of nonces:**

```cpp
// In mine_challenge(), e.g. inside the nonce loop, every 2048 nonces or every 3 seconds:
void send_progress(const char* challenge_id, const char* hash) {
    JsonDocument doc;
    doc["device_id"] = g_device_id;
    doc["challenge_id"] = challenge_id;
    doc["hash"] = hash;
    String body;
    serializeJson(doc, body);
    String resp;
    if (rpc_post("/devices/progress", body, resp)) {
        // optional: log or ignore
    }
}
```

**What to pass as `hash`:** For example:

- The **tensor hash** of the current output (you already have `compute_tensor_hash()`), or  
- A hash of **challenge_id + current nonce** (or nonce range), or  
- The same **proof hash** you will submit when done (update it as the nonce advances).

Example (pseudocode) using current nonce and challenge_id:

```cpp
// Every 2–5 s or every K nonces in the mining loop:
char progress_hash[65];
char buf[128];
snprintf(buf, sizeof(buf), "%s%llu", challenge_id, (unsigned long long)current_nonce);
// Then SHA256(buf) -> hex into progress_hash
send_progress(challenge_id, progress_hash);
```

### 2.3 RPC base URL and TLS

Configure the validator base URL in `include/pot_o_config.h` or via PlatformIO build flags (see `firmware/esp-pot-o-miner/README.md`). Default is `pot.rpc.gateway.tribewarez.com`. Use the same host for `/devices/register`, `/challenge`, `/devices/progress`, and `/submit`.

### 2.4 Summary for ESP

| Step            | Endpoint              | When / what |
|-----------------|-----------------------|-------------|
| Register        | `POST /devices/register` | Once at boot; `device_id` = MAC, `device_type` = `esp32s` / `esp8266`. |
| Get challenge   | `POST /challenge`     | Each round; body `{"device_type":"esp32s"}` or `"esp8266"`. |
| Progress        | `POST /devices/progress` | Every few seconds while mining; `device_id`, `challenge_id`, `hash`. |
| Submit          | `POST /submit`        | When proof is found; include `proof`, `device_id`, `device_type`. |

### 2.5 Self-check for firmware updates (ESP / AIoT devices)

On boot (after WiFi comes up), ESP devices can ask the central gateway if a newer firmware is available.

**HTTP request (from device or gateway-side helper):**

```text
GET https://status.rpc.gateway.tribewarez.com/api/device/self-check
  ?device_type=esp32
  &channel=testnet
  &kind=firmware
  &current_version=1.0.0
```

Key fields:

- `device_type`: `esp32` or `esp8266` (the server normalizes `esp32s` → `esp32`).
- `channel`: network track (`testnet`, `mainnet`, etc.).
- `kind`: `"firmware"` for on-device firmware.
- `current_version`: firmware version compiled into the image (e.g. `POT_O_VERSION`).

The response mirrors the CLI example above and includes:

- `latest_version`, `min_supported_version`
- `up_to_date`, `update_required`, `update_recommended`
- Optional `artifact` with `download_url`, `checksum`, `size_bytes`

Suggested behavior for firmware:

- If `update_required` is true, stop mining and surface an error (old firmware is no longer supported).
- If `update_recommended` is true and `artifact.download_url` is present, schedule a background download and staged flash according to the device’s capabilities.
- If `up_to_date` is true, proceed with the normal mining loop.

---

## 3. Reference

- **Device registry and keys:** [DEVICES.md](./DEVICES.md)  
- **ESP firmware:** [firmware/esp-pot-o-miner/README.md](./firmware/esp-pot-o-miner/README.md)  
- **Validator keys (relayer):** [keys/README.md](./keys/README.md)
