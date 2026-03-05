// SHA-256: platform-specific implementation (ESP32 mbedtls + HW when CONFIG_MBEDTLS_HARDWARE_SHA, ESP8266 BearSSL)

#include "pot_o/sha256_util.h"
#include <string.h>
#include <stdio.h>

#if defined(ESP32S_DEVICE)
  #include <mbedtls/sha256.h>
  /* mbedtls uses ESP32 hardware SHA when CONFIG_MBEDTLS_HARDWARE_SHA=y (e.g. via sdkconfig.defaults) */
  struct SHA256Ctx { mbedtls_sha256_context ctx; };

  void sha256_raw(const uint8_t* data, size_t len, uint8_t out[32]) {
      mbedtls_sha256_context ctx;
      mbedtls_sha256_init(&ctx);
      mbedtls_sha256_starts(&ctx, 0);
      mbedtls_sha256_update(&ctx, data, len);
      mbedtls_sha256_finish(&ctx, out);
      mbedtls_sha256_free(&ctx);
  }

  void sha256_init(SHA256Ctx* c) {
      mbedtls_sha256_init(&c->ctx);
      mbedtls_sha256_starts(&c->ctx, 0);
  }
  void sha256_update(SHA256Ctx* c, const uint8_t* data, size_t len) {
      mbedtls_sha256_update(&c->ctx, data, len);
  }
  void sha256_finish(SHA256Ctx* c, uint8_t out[32]) {
      mbedtls_sha256_finish(&c->ctx, out);
      mbedtls_sha256_free(&c->ctx);
  }
#else
  /* ESP8266 */
  #include <Hash.h>
  struct SHA256Ctx { br_sha256_context ctx; };

  void sha256_raw(const uint8_t* data, size_t len, uint8_t out[32]) {
      br_sha256_context ctx;
      br_sha256_init(&ctx);
      br_sha256_update(&ctx, data, len);
      br_sha256_out(&ctx, out);
  }

  void sha256_init(SHA256Ctx* c) {
      br_sha256_init(&c->ctx);
  }
  void sha256_update(SHA256Ctx* c, const uint8_t* data, size_t len) {
      br_sha256_update(&c->ctx, data, len);
  }
  void sha256_finish(SHA256Ctx* c, uint8_t out[32]) {
      br_sha256_out(&c->ctx, out);
  }
#endif

void bytes_to_hex(const uint8_t* in, size_t len, char* out) {
    for (size_t i = 0; i < len; i++)
        sprintf(out + i * 2, "%02x", in[i]);
    out[len * 2] = '\0';
}

int hex_to_bytes(const char* hex, uint8_t* out, size_t max_len) {
    size_t hex_len = strlen(hex);
    size_t byte_len = hex_len / 2;
    if (byte_len > max_len) byte_len = max_len;
    for (size_t i = 0; i < byte_len; i++) {
        unsigned int byte;
        sscanf(hex + i * 2, "%2x", &byte);
        out[i] = (uint8_t)byte;
    }
    return (int)byte_len;
}
