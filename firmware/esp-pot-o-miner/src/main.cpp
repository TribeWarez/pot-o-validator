// PoT-O Miner Firmware for ESP32-S / ESP8266
// Connects to pot.rpc.gateway.tribewarez.com, fetches challenges,
// runs tensor operations, and submits proofs.
// Uses ed25519 challenge-response auth (Tribechain).

#include <Arduino.h>
#include <ArduinoJson.h>

#if defined(ESP32S_DEVICE)
  #include <WiFi.h>
  #include <HTTPClient.h>
  #include <WiFiClientSecure.h>
  #include <Preferences.h>
#else
  #include <ESP8266WiFi.h>
  #include <ESP8266HTTPClient.h>
  #include <WiFiClientSecureBearSSL.h>
  #include <Preferences.h>
#endif

#include <Crypto.h>
#include <Ed25519.h>

#include "pot_o/pot_o_config.h"
#include "pot_o/tensor_ops.h"
#include "pot_o/sha256_util.h"
#include "pot_o/neural_path.h"
#include "pot_o/mml_score.h"

// ── Ed25519 auth globals ────────────────────────────────────────────────────

static uint8_t g_privkey[32];
static uint8_t g_pubkey[32];
static bool g_have_keypair = false;
static String g_auth_token;

static const char* NVS_NS = "pot-o";
static const char* NVS_PK = "privkey";
static const char* NVS_PUB = "pubkey";

// ── Hex helpers ─────────────────────────────────────────────────────────────

static void hex_encode(const uint8_t* data, size_t len, char* out) {
    static const char hex[] = "0123456789abcdef";
    for (size_t i = 0; i < len; i++) {
        out[i * 2] = hex[data[i] >> 4];
        out[i * 2 + 1] = hex[data[i] & 0x0f];
    }
    out[len * 2] = '\0';
}

static bool hex_decode(const char* s, uint8_t* out, size_t out_len) {
    size_t slen = strlen(s);
    if (slen != out_len * 2) return false;
    for (size_t i = 0; i < out_len; i++) {
        char c1 = s[i * 2], c2 = s[i * 2 + 1];
        uint8_t h = (c1 >= 'a') ? (c1 - 'a' + 10) : (c1 - '0');
        uint8_t l = (c2 >= 'a') ? (c2 - 'a' + 10) : (c2 - '0');
        out[i] = (h << 4) | l;
    }
    return true;
}

// ── Globals ─────────────────────────────────────────────────────────────────

static float g_input_buf[MAX_TENSOR_DIM * MAX_TENSOR_DIM];
static float g_output_buf[MAX_TENSOR_DIM * MAX_TENSOR_DIM];

struct MinerStats {
    uint32_t challenges_fetched;
    uint32_t proofs_found;
    uint32_t proofs_submitted;
    uint32_t proofs_accepted;
    unsigned long uptime_start;
};
static MinerStats stats = {0, 0, 0, 0, 0};

static String g_device_id;
static unsigned long g_last_heartbeat = 0;

// ── Forward declarations ────────────────────────────────────────────────────

void wifi_connect();
String rpc_url(const char* path);
String bootstrap_url(const char* path);
bool rpc_post(const char* path, const String& body, String& response);
bool rpc_get(const char* path, String& response);
void init_crypto();
bool auth_handshake();
bool register_device();
bool discover_bootstrap_peers();
bool fetch_challenge(JsonDocument& challenge_doc);
bool mine_challenge(const JsonDocument& challenge, JsonDocument& proof_doc);
bool submit_proof(const JsonDocument& proof);
void compute_proof_hash(const char* challenge_id, const char* tensor_hash,
                        double mml_score, const char* path_sig,
                        uint64_t nonce, char* hash_out);

// ── Setup ───────────────────────────────────────────────────────────────────

void setup() {
    Serial.begin(115200);
    delay(100);
    Serial.println();
    Serial.println(F("========================================"));
    Serial.println(F("  PoT-O Miner - Tribewarez Testnet"));
    Serial.print(F("  Device: "));
#if defined(ESP32S_DEVICE)
    Serial.println(F("ESP32-S (64x64, 320KB)"));
#else
    Serial.println(F("ESP8266 (32x32, 80KB)"));
#endif
    Serial.println(F("========================================"));

    stats.uptime_start = millis();
    wifi_connect();
    init_crypto();
    if (auth_handshake()) {
        Serial.println(F("[AUTH] Session established"));
    } else {
        Serial.println(F("[AUTH] Auth failed — mining will proceed without auth (limited)"));
    }
    register_device();
#ifdef POT_BOOTSTRAP_URL
    discover_bootstrap_peers();
#endif
}

// ── Main loop ───────────────────────────────────────────────────────────────

void loop() {
    if (WiFi.status() != WL_CONNECTED) {
        Serial.println(F("[WIFI] Reconnecting..."));
        wifi_connect();
    }

    // Heartbeat
    if (millis() - g_last_heartbeat > HEARTBEAT_INTERVAL_MS) {
        g_last_heartbeat = millis();
        String resp;
        if (rpc_get("/health", resp)) {
            Serial.println(F("[HEARTBEAT] RPC healthy"));
        } else {
            Serial.println(F("[HEARTBEAT] RPC unreachable, retrying next cycle"));
            delay(5000);
            return;
        }
    }

    // Fetch challenge
    Serial.println(F("\n[MINE] Fetching challenge..."));
    JsonDocument challenge_doc;
    if (!fetch_challenge(challenge_doc)) {
        Serial.println(F("[MINE] Failed to fetch challenge, waiting 10s"));
        delay(10000);
        return;
    }
    stats.challenges_fetched++;

    const char* op = challenge_doc["operation_type"] | "relu";
    int op_id = op_from_name(op);
    Serial.printf("[MINE] Challenge: op=%s dim=%d difficulty=%llu\n",
                  op,
                  (int)(challenge_doc["max_tensor_dim"] | MAX_TENSOR_DIM),
                  (unsigned long long)(challenge_doc["difficulty"] | 2));

    if (!op_supported(op_id)) {
        Serial.printf("[MINE] Operation '%s' not supported on this device, skipping\n", op);
        delay(2000);
        return;
    }

    // Mine
    JsonDocument proof_doc;
    if (mine_challenge(challenge_doc, proof_doc)) {
        stats.proofs_found++;
        Serial.println(F("[MINE] Proof found! Submitting..."));
        if (submit_proof(proof_doc)) {
            stats.proofs_accepted++;
            Serial.println(F("[MINE] Proof ACCEPTED"));
        } else {
            Serial.println(F("[MINE] Proof rejected or submission failed"));
        }
        stats.proofs_submitted++;
    } else {
        Serial.println(F("[MINE] No proof found within iteration limit"));
    }

    Serial.printf("[STATS] challenges=%u found=%u submitted=%u accepted=%u uptime=%lus\n",
                  stats.challenges_fetched, stats.proofs_found,
                  stats.proofs_submitted, stats.proofs_accepted,
                  (millis() - stats.uptime_start) / 1000);

    delay(1000);
}

// ── WiFi ────────────────────────────────────────────────────────────────────

void wifi_connect() {
    Serial.printf("[WIFI] Connecting to %s", WIFI_SSID);
    WiFi.mode(WIFI_STA);
    WiFi.begin(WIFI_SSID, WIFI_PASS);
    int attempts = 0;
    while (WiFi.status() != WL_CONNECTED && attempts < 60) {
        delay(500);
        Serial.print(".");
        attempts++;
    }
    if (WiFi.status() == WL_CONNECTED) {
        Serial.printf("\n[WIFI] Connected, IP: %s\n", WiFi.localIP().toString().c_str());
    } else {
        Serial.println(F("\n[WIFI] Connection failed, will retry"));
    }
}

// ── HTTP helpers ────────────────────────────────────────────────────────────

String rpc_url(const char* path) {
#if POT_RPC_TLS
    return String("https://") + POT_RPC_HOST + path;
#else
    return String("http://") + POT_RPC_HOST + ":" + String(POT_RPC_PORT) + path;
#endif
}

#ifdef POT_BOOTSTRAP_URL
String bootstrap_url(const char* path) {
    return String("https://") + POT_BOOTSTRAP_URL + ":" + String(POT_BOOTSTRAP_PORT) + path;
}
#endif

bool rpc_post(const char* path, const String& body, String& response) {
    HTTPClient http;
#if defined(ESP32S_DEVICE)
    WiFiClientSecure client;
    client.setInsecure(); // testnet - skip cert validation
    http.begin(client, rpc_url(path));
#else
    std::unique_ptr<BearSSL::WiFiClientSecure> client(new BearSSL::WiFiClientSecure);
    client->setInsecure();
    http.begin(*client, rpc_url(path));
#endif
    http.addHeader("Content-Type", "application/json");
    if (g_auth_token.length() > 0) {
        http.addHeader("Authorization", "Bearer " + g_auth_token);
    }
    http.setTimeout(15000);

    int code = http.POST(body);
    if (code > 0) {
        response = http.getString();
        http.end();
        return code >= 200 && code < 300;
    }
    http.end();
    return false;
}

bool rpc_get(const char* path, String& response) {
    HTTPClient http;
#if defined(ESP32S_DEVICE)
    WiFiClientSecure client;
    client.setInsecure();
    http.begin(client, rpc_url(path));
#else
    std::unique_ptr<BearSSL::WiFiClientSecure> client(new BearSSL::WiFiClientSecure);
    client->setInsecure();
    http.begin(*client, rpc_url(path));
#endif
    http.setTimeout(10000);

    int code = http.GET();
    if (code > 0) {
        response = http.getString();
        http.end();
        return code >= 200 && code < 300;
    }
    http.end();
    return false;
}

// ── Ed25519 keypair (NVS) ───────────────────────────────────────────────────

static void init_crypto() {
    Preferences prefs;
    prefs.begin(NVS_NS, false);

    bool have = prefs.isKey(NVS_PK) && prefs.isKey(NVS_PUB);
    if (have) {
        size_t pk_len = prefs.getBytes(NVS_PK, g_privkey, 32);
        size_t pu_len = prefs.getBytes(NVS_PUB, g_pubkey, 32);
        have = (pk_len == 32 && pu_len == 32);
    }

    if (!have) {
        Serial.println(F("[CRYPTO] Generating ed25519 keypair..."));
        Ed25519::generatePrivateKey(g_privkey);
        Ed25519::derivePublicKey(g_pubkey, g_privkey);
        prefs.putBytes(NVS_PK, g_privkey, 32);
        prefs.putBytes(NVS_PUB, g_pubkey, 32);
        Serial.println(F("[CRYPTO] Keypair saved to NVS"));
    } else {
        Serial.println(F("[CRYPTO] Loaded keypair from NVS"));
    }
    prefs.end();
    g_have_keypair = true;

    char hex[65];
    hex_encode(g_pubkey, 32, hex);
    Serial.printf("[CRYPTO] Pubkey: %s\n", hex);
}

// ── Auth handshake (challenge-response) ─────────────────────────────────────

static bool auth_handshake() {
    if (!g_have_keypair) {
        Serial.println(F("[AUTH] No keypair, skipping auth"));
        return false;
    }

    char pubkey_hex[65];
    hex_encode(g_pubkey, 32, pubkey_hex);

    // Step 1: POST /auth/challenge
    Serial.println(F("[AUTH] Requesting challenge..."));
    String chal_body = String("{\"pubkey\":\"") + pubkey_hex + "\"}";
    String chal_resp;
    if (!rpc_post("/auth/challenge", chal_body, chal_resp)) {
        Serial.println(F("[AUTH] Challenge request failed"));
        return false;
    }

    JsonDocument doc;
    DeserializationError err = deserializeJson(doc, chal_resp);
    if (err) {
        Serial.printf("[AUTH] JSON error: %s\n", err.c_str());
        return false;
    }

    const char* nonce_hex = doc["nonce"] | "";
    if (strlen(nonce_hex) != 64) {
        Serial.println(F("[AUTH] Invalid nonce"));
        return false;
    }

    uint8_t nonce[32];
    if (!hex_decode(nonce_hex, nonce, 32)) {
        Serial.println(F("[AUTH] Nonce decode failed"));
        return false;
    }

    // Step 2: sign nonce with ed25519
    uint8_t sig[64];
    Ed25519::sign(sig, g_privkey, g_pubkey, nonce, 32);

    char sig_hex[129];
    hex_encode(sig, 64, sig_hex);

    // Step 3: POST /auth/verify
    String ver_body = String("{\"pubkey\":\"") + pubkey_hex +
                      "\",\"signature\":\"" + sig_hex +
                      "\",\"message\":\"" + nonce_hex + "\"}";
    String ver_resp;
    if (!rpc_post("/auth/verify", ver_body, ver_resp)) {
        Serial.println(F("[AUTH] Verify request failed"));
        return false;
    }

    JsonDocument vdoc;
    err = deserializeJson(vdoc, ver_resp);
    if (err) {
        Serial.printf("[AUTH] Verify JSON error: %s\n", err.c_str());
        return false;
    }

    const char* token = vdoc["token"] | "";
    if (strlen(token) == 0) {
        Serial.println(F("[AUTH] No token in response"));
        return false;
    }

    g_auth_token = token;
    Serial.println(F("[AUTH] Authenticated successfully"));
    return true;
}

// ── Device registration ─────────────────────────────────────────────────────

bool register_device() {
#if defined(ESP32S_DEVICE)
    const char* dev_type = "esp32s";
#else
    const char* dev_type = "esp8266";
#endif
    String body = String("{\"device_type\":\"") + dev_type +
                  "\",\"device_id\":\"" + WiFi.macAddress() + "\"}";
    String resp;
    if (rpc_post("/devices/register", body, resp)) {
        JsonDocument doc;
        deserializeJson(doc, resp);
        g_device_id = doc["device_id"].as<String>();
        Serial.printf("[REG] Registered as %s (id: %s)\n", dev_type, g_device_id.c_str());
        return true;
    }
    Serial.println(F("[REG] Registration failed (non-fatal, continuing)"));
    g_device_id = WiFi.macAddress();
    return false;
}

// ── Bootstrap peer discovery ────────────────────────────────────────────────

#ifdef POT_BOOTSTRAP_URL
bool discover_bootstrap_peers() {
    Serial.printf("[BOOTSTRAP] Discovering peers via %s\n", POT_BOOTSTRAP_URL);
    String resp;
    HTTPClient http;
#if defined(ESP32S_DEVICE)
    WiFiClientSecure client;
    client.setInsecure();
    http.begin(client, bootstrap_url("/peers"));
#else
    std::unique_ptr<BearSSL::WiFiClientSecure> client(new BearSSL::WiFiClientSecure);
    client->setInsecure();
    http.begin(*client, bootstrap_url("/peers"));
#endif
    http.setTimeout(10000);

    int code = http.GET();
    if (code > 0) {
        resp = http.getString();
        http.end();
        if (code >= 200 && code < 300) {
            JsonDocument doc;
            DeserializationError err = deserializeJson(doc, resp);
            if (err) {
                Serial.printf("[BOOTSTRAP] JSON parse error: %s\n", err.c_str());
                return false;
            }
            JsonArray peers = doc["peers"].as<JsonArray>();
            int count = 0;
            for (JsonVariant v : peers) {
                const char* host = v["host"] | "";
                int port = v["port"] | 8900;
                if (strlen(host) > 0) {
                    Serial.printf("[BOOTSTRAP] Peer %d: %s:%d\n", ++count, host, port);
                }
            }
            Serial.printf("[BOOTSTRAP] Discovery complete: %d peer(s) found\n", count);
            return true;
        }
    }
    http.end();
    Serial.println(F("[BOOTSTRAP] Discovery failed (non-fatal)"));
    return false;
}
#endif

// ── Challenge fetch ─────────────────────────────────────────────────────────

bool fetch_challenge(JsonDocument& challenge_doc) {
#if defined(ESP32S_DEVICE)
    const char* dev_type = "esp32s";
#else
    const char* dev_type = "esp8266";
#endif
    String body = String("{\"device_type\":\"") + dev_type + "\"}";
    String resp;
    if (!rpc_post("/challenge", body, resp)) return false;

    DeserializationError err = deserializeJson(challenge_doc, resp);
    if (err) {
        Serial.printf("[CHALLENGE] JSON parse error: %s\n", err.c_str());
        return false;
    }
    return challenge_doc.containsKey("id");
}

// ── Proof hash computation ──────────────────────────────────────────────────
// Must match PotOConsensus::compute_proof_hash exactly:
// SHA256(challenge_id_bytes || tensor_hash_bytes || mml_score_le || path_sig_bytes || nonce_le)

void compute_proof_hash(const char* challenge_id, const char* tensor_hash,
                        double mml_score, const char* path_sig,
                        uint64_t nonce, char* hash_out) {
    SHA256Ctx ctx;
    sha256_init(&ctx);

    sha256_update(&ctx, (const uint8_t*)challenge_id, strlen(challenge_id));
    sha256_update(&ctx, (const uint8_t*)tensor_hash, strlen(tensor_hash));

    // mml_score as f64 LE bytes
    uint8_t score_bytes[8];
    memcpy(score_bytes, &mml_score, 8);
    sha256_update(&ctx, score_bytes, 8);

    sha256_update(&ctx, (const uint8_t*)path_sig, strlen(path_sig));

    // nonce as u64 LE bytes
    uint8_t nonce_bytes[8];
    memcpy(nonce_bytes, &nonce, 8);
    sha256_update(&ctx, nonce_bytes, 8);

    uint8_t hash[32];
    sha256_finish(&ctx, hash);
    bytes_to_hex(hash, 32, hash_out);
}

// ── Tensor hash (same as Tensor::calculate_hash) ────────────────────────────

static void compute_tensor_hash(const float* data, size_t len,
                                size_t rows, size_t cols,
                                char* hash_out) {
    SHA256Ctx ctx;
    sha256_init(&ctx);
    // Data bytes (f32 LE)
    sha256_update(&ctx, (const uint8_t*)data, len * sizeof(float));
    // Shape dims as usize (on server = u64 LE)
    uint64_t dim;
    dim = (uint64_t)rows;
    sha256_update(&ctx, (const uint8_t*)&dim, sizeof(dim));
    dim = (uint64_t)cols;
    sha256_update(&ctx, (const uint8_t*)&dim, sizeof(dim));

    uint8_t hash[32];
    sha256_finish(&ctx, hash);
    bytes_to_hex(hash, 32, hash_out);
}

// ── Mining ──────────────────────────────────────────────────────────────────

bool mine_challenge(const JsonDocument& challenge, JsonDocument& proof_doc) {
    const char* challenge_id = challenge["id"];
    const char* op_name = challenge["operation_type"] | "relu";
    double mml_threshold = challenge["mml_threshold"] | 0.85;
    uint32_t path_distance_max = challenge["path_distance_max"] | 8;
    size_t max_dim = challenge["max_tensor_dim"] | MAX_TENSOR_DIM;

    // Clamp to device limits
    if (max_dim > MAX_TENSOR_DIM) max_dim = MAX_TENSOR_DIM;

    // Deserialize input tensor from challenge JSON
    JsonArrayConst tensor_data = challenge["input_tensor"]["data"]["F32"];
    size_t dim = 0;
    JsonArrayConst shape_dims = challenge["input_tensor"]["shape"]["dims"];
    size_t rows = 1, cols = 1;
    int shape_idx = 0;
    for (JsonVariantConst d : shape_dims) {
        size_t v = d.as<size_t>();
        if (v > max_dim) v = max_dim;
        if (shape_idx == 0) rows = v;
        if (shape_idx == 1) cols = v;
        shape_idx++;
    }
    dim = rows * cols;
    if (dim > MAX_TENSOR_DIM * MAX_TENSOR_DIM) dim = MAX_TENSOR_DIM * MAX_TENSOR_DIM;

    size_t i = 0;
    for (JsonVariantConst v : tensor_data) {
        if (i >= dim) break;
        g_input_buf[i++] = v.as<float>();
    }
    // Pad with golden ratio if needed
    while (i < dim) {
        float seed = (float)i * 0.61803399f;
        g_input_buf[i] = seed - floorf(seed);
        i++;
    }

    // Execute tensor operation (measure kernel time)
    int op_id = op_from_name(op_name);
    Tensor in_tensor;
    tensor_init(&in_tensor, g_input_buf, rows, cols);
    unsigned long op_start_us = micros();
    size_t out_len = tensor_execute(op_id, &in_tensor, g_output_buf);
    unsigned long op_end_us = micros();

    // Compute MML score
    double mml_score = compute_mml_score(g_input_buf, dim, g_output_buf, out_len);
    bool mml_ok = mml_score <= mml_threshold;

    Serial.printf("[MINE] Tensor op=%s rows=%u cols=%u in_len=%u out_len=%u op_time_us=%lu\n",
                  op_name,
                  (unsigned int)rows,
                  (unsigned int)cols,
                  (unsigned int)dim,
                  (unsigned int)out_len,
                  (unsigned long)(op_end_us - op_start_us));

    Serial.printf("[MINE] MML score=%.4f threshold=%.4f %s\n",
                  mml_score, mml_threshold, mml_ok ? "PASS" : "FAIL");

    if (!mml_ok) {
        // MML doesn't pass; no point iterating nonces
        return false;
    }

    // Compute expected neural path
    uint8_t exp_path[NEURAL_TOTAL_NEURONS];
    size_t exp_len;
    expected_path(challenge_id, exp_path, &exp_len);

    // Compute tensor result hash
    char tensor_hash[65];
    compute_tensor_hash(g_output_buf, out_len, rows, cols, tensor_hash);

    // Nonce search (measure mining loop time)
    unsigned long mine_start_us = micros();
    for (uint64_t nonce = 0; nonce < MAX_MINE_ITERATIONS; nonce++) {
        uint8_t actual_path[NEURAL_TOTAL_NEURONS];
        size_t actual_len;
        compute_actual_path(g_output_buf, out_len, nonce, actual_path, &actual_len);

        size_t cmp_len = actual_len < exp_len ? actual_len : exp_len;
        uint32_t dist = hamming_distance(actual_path, exp_path, cmp_len);

        if (dist <= path_distance_max) {
            char path_sig[NEURAL_TOTAL_NEURONS / 4 + 4];
            path_to_hex(actual_path, actual_len, path_sig);

            char comp_hash[65];
            compute_proof_hash(challenge_id, tensor_hash,
                               mml_score, path_sig, nonce, comp_hash);

            unsigned long mine_end_us = micros();
            Serial.printf("[MINE] Found proof at nonce=%llu dist=%u mine_time_us=%lu\n",
                          (unsigned long long)nonce,
                          dist,
                          (unsigned long)(mine_end_us - mine_start_us));

            // Build proof JSON
            proof_doc["challenge_id"] = challenge_id;
            proof_doc["challenge_hash"] = challenge["slot_hash"] | "";
            proof_doc["tensor_result_hash"] = tensor_hash;
            proof_doc["mml_score"] = mml_score;
            proof_doc["path_signature"] = path_sig;
            proof_doc["path_distance"] = dist;
            proof_doc["computation_nonce"] = nonce;
            proof_doc["computation_hash"] = comp_hash;
            proof_doc["miner_pubkey"] = MINER_PUBKEY;
            proof_doc["timestamp"] = "2026-03-01T00:00:00Z";

            return true;
        }

        if (nonce % 1000 == 0 && nonce > 0) {
            Serial.printf("[MINE] nonce=%llu best_dist=%u (need<=%u)\n",
                          (unsigned long long)nonce, dist, path_distance_max);
            yield(); // prevent WDT reset
        }
    }

    unsigned long mine_end_us = micros();
    Serial.printf("[MINE] Exhausted nonces without proof, mine_time_us=%lu\n",
                  (unsigned long)(mine_end_us - mine_start_us));
    return false;
}

// ── Proof submission ────────────────────────────────────────────────────────

bool submit_proof(const JsonDocument& proof) {
    JsonDocument submit_doc;
    submit_doc["proof"] = proof;

    String body;
    serializeJson(submit_doc, body);
    String resp;
    if (rpc_post("/submit", body, resp)) {
        JsonDocument resp_doc;
        deserializeJson(resp_doc, resp);
        bool accepted = resp_doc["accepted"] | false;
        if (accepted) {
            const char* tx = resp_doc["tx_signature"] | "n/a";
            Serial.printf("[SUBMIT] tx_signature: %s\n", tx);
        }
        return accepted;
    }
    return false;
}
