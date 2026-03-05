#ifndef SHA256_UTIL_H
#define SHA256_UTIL_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque context; platform-specific implementation in sha256_util_esp32.c / sha256_util_esp8266.c
typedef struct SHA256Ctx SHA256Ctx;

void sha256_raw(const uint8_t* data, size_t len, uint8_t out[32]);

void sha256_init(SHA256Ctx* c);
void sha256_update(SHA256Ctx* c, const uint8_t* data, size_t len);
void sha256_finish(SHA256Ctx* c, uint8_t out[32]);

void bytes_to_hex(const uint8_t* in, size_t len, char* out);
int hex_to_bytes(const char* hex, uint8_t* out, size_t max_len);

#ifdef __cplusplus
}
#endif

#endif // SHA256_UTIL_H
