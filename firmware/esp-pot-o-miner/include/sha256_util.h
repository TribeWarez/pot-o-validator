#ifndef SHA256_UTIL_H
#define SHA256_UTIL_H

// Lightweight SHA-256 for ESP (uses mbedtls which ships with ESP-IDF/Arduino)
#if defined(ESP32S_DEVICE)
  #include <mbedtls/sha256.h>
#else
  // ESP8266 Arduino core includes a Hash library
  #include <Hash.h>
#endif

#include <stdint.h>
#include <string.h>
#include <stdio.h>

static inline void sha256_raw(const uint8_t* data, size_t len, uint8_t out[32]) {
#if defined(ESP32S_DEVICE)
    mbedtls_sha256_context ctx;
    mbedtls_sha256_init(&ctx);
    mbedtls_sha256_starts(&ctx, 0);
    mbedtls_sha256_update(&ctx, data, len);
    mbedtls_sha256_finish(&ctx, out);
    mbedtls_sha256_free(&ctx);
#else
    // ESP8266 sha256 via BearSSL
    br_sha256_context ctx;
    br_sha256_init(&ctx);
    br_sha256_update(&ctx, data, len);
    br_sha256_out(&ctx, out);
#endif
}

// Multi-part SHA-256 helper
typedef struct {
#if defined(ESP32S_DEVICE)
    mbedtls_sha256_context ctx;
#else
    br_sha256_context ctx;
#endif
} SHA256Ctx;

static inline void sha256_init(SHA256Ctx* c) {
#if defined(ESP32S_DEVICE)
    mbedtls_sha256_init(&c->ctx);
    mbedtls_sha256_starts(&c->ctx, 0);
#else
    br_sha256_init(&c->ctx);
#endif
}

static inline void sha256_update(SHA256Ctx* c, const uint8_t* data, size_t len) {
#if defined(ESP32S_DEVICE)
    mbedtls_sha256_update(&c->ctx, data, len);
#else
    br_sha256_update(&c->ctx, data, len);
#endif
}

static inline void sha256_finish(SHA256Ctx* c, uint8_t out[32]) {
#if defined(ESP32S_DEVICE)
    mbedtls_sha256_finish(&c->ctx, out);
    mbedtls_sha256_free(&c->ctx);
#else
    br_sha256_out(&c->ctx, out);
#endif
}

static inline void bytes_to_hex(const uint8_t* in, size_t len, char* out) {
    for (size_t i = 0; i < len; i++) {
        sprintf(out + i * 2, "%02x", in[i]);
    }
    out[len * 2] = '\0';
}

static inline int hex_to_bytes(const char* hex, uint8_t* out, size_t max_len) {
    size_t hex_len = strlen(hex);
    size_t byte_len = hex_len / 2;
    if (byte_len > max_len) byte_len = max_len;
    for (size_t i = 0; i < byte_len; i++) {
        unsigned int byte;
        sscanf(hex + i * 2, "%2x", &byte);
        out[i] = (uint8_t)byte;
    }
    return byte_len;
}

#endif // SHA256_UTIL_H
