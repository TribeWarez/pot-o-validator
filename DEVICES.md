# Device registry and submit behavior

## Registry key

The device registry (persisted in `device_registry.json`) keys entries as follows:

- **When the client sends `device_id`** on `POST /submit` or `POST /devices/register`: the registry key is that `device_id` (e.g. MAC or UUID). One entry per device; multiple miner pubkeys can submit with the same `device_id` and all update the same entry.
- **When the client omits `device_id`** on `POST /submit`: the registry key is `{miner_pubkey}:{device_type}`. So **omitting `device_id` implies one registry entry per (pubkey, device_type)**. One physical machine running multiple calculation threads or miner pubkeys will create multiple registry entries and appear as multiple “devices” in `GET /devices` and in the status dashboard.

## Multiple threads or pubkeys on the same device

- For a single device running multiple threads or multiple miner pubkeys, send a **stable `device_id`** (e.g. from `POST /devices/register` or machine MAC) on every `POST /submit` so the registry stays one entry per machine and dashboard counts reflect real device count.
- If you omit `device_id`, each (pubkey, device_type) is treated as a separate device; no change is required for correctness, but device count will be inflated.
- Every device or thread should send the **current running calculation with hash in real time** so the status dashboard and validators can reflect live activity. Use **`POST /devices/progress`** (see below).

## POST /devices/progress (current calculation with hash in real time)

Devices and threads should call this endpoint to report the current running calculation and its hash. The validator stores it per device and exposes it in `GET /devices` (`devices_detail[].current_calculation`) and the status dashboard uses it for live activity.

**Request body (JSON):**

- `challenge_id` (required): ID of the challenge being worked on.
- `hash` (required): Hash of the current running calculation (e.g. state or work-in-progress).
- `device_id` (optional): If set, this device entry is updated (same key as in submit/register).
- `miner_pubkey` (optional): If `device_id` is omitted, used with `device_type` to form the registry key `{miner_pubkey}:{device_type}`.
- `device_type` (optional): Default `"native"`. Used with `miner_pubkey` when `device_id` is not set.

**Response:** `200` with `{ "ok": true, "updated": true }`. `400` if neither `device_id` nor `miner_pubkey` is set.

Call this at a regular interval (e.g. every few seconds) while a calculation is running so the dashboard shows up-to-date activity.

## Per-device miner pubkeys (analytics)

When submissions include `device_id`, the validator records up to 100 distinct miner pubkeys per device in the optional `miner_pubkeys` field. `GET /devices` returns a `devices_detail` map (device_id → `device_type`, `proofs_valid`, `tasks_processed`, `last_activity`, `miner_pubkeys`) for analytics.

**CLI and ESP usage:** See [CLI_AND_ESP_GUIDE.md](./CLI_AND_ESP_GUIDE.md) for curl examples and ESP firmware integration (register, progress, submit).

## Multi-validator

If a status dashboard aggregates multiple PoT-O validators, device IDs are not globally unique across validators. When merging or displaying combined device stats, namespace device keys by validator (e.g. endpoint or node_id) to avoid double-counting the same physical device that connects to more than one validator.
