#ifndef __POSEMESH_C_CONFIG_H__
#define __POSEMESH_C_CONFIG_H__

#if defined(__APPLE__)
#    if defined(POSEMESH_BUILD)
#        define PSM_API __attribute__((visibility("default")))
#    else
#        define PSM_API
#    endif
#else
#    error "Platform not supported."
#endif

#endif // __POSEMESH_C_CONFIG_H__
