#ifndef __POSEMESH_NETWORKING_API_H__
#define __POSEMESH_NETWORKING_API_H__

typedef struct psm_posemesh_networking_context psm_posemesh_networking_context_t;

#if defined(__cplusplus)
extern "C" {
#endif

psm_posemesh_networking_context_t* psm_posemesh_networking_context_create();
void psm_posemesh_networking_context_destroy(psm_posemesh_networking_context_t* context);

#if defined(__cplusplus)
}
#endif

#endif // __POSEMESH_NETWORKING_API_H__
