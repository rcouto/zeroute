#ifndef ZEROUTE_ENGINE_H
#define ZEROUTE_ENGINE_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    void (*put)(void *, int, uint64_t, uint64_t, const uint8_t *, int);
    int (*get)(void *, int, uint64_t, uint64_t, uint8_t *, unsigned int);
    int (*send)(void *, int, const uint8_t *, unsigned int, unsigned int, const uint8_t *, unsigned int);
    void (*event)(void *, int);
    void (*configuration)(void *, uint64_t, int, uint64_t, unsigned int, const char *);
    void (*frame)(void *, uint64_t, uint64_t, uint64_t, unsigned int, const uint8_t *, unsigned int);
    int (*lookup_path)(void *, uint64_t, int, uint8_t *, unsigned int *);
} zr_callbacks;

uint64_t zr_core_version(void);
void *zr_node_new(const zr_callbacks *, void *, int64_t);
void zr_node_delete(void *);
uint64_t zr_node_address(void *);
int zr_node_join(void *, uint64_t);
int zr_node_receive(void *, int64_t, int, const uint8_t *, unsigned int, unsigned int, const uint8_t *, unsigned int, int64_t *);
int zr_node_frame(void *, int64_t, uint64_t, uint64_t, uint64_t, unsigned int, const uint8_t *, unsigned int, int64_t *);
int zr_node_subscribe(void *, uint64_t, uint64_t, unsigned int, int);
int zr_node_background(void *, int64_t, int64_t *);
#ifdef __cplusplus
}
#endif
#endif
