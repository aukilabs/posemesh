#ifndef __POSEMESH_C_API_H__
#define __POSEMESH_C_API_H__

#if defined(__APPLE__)
#    if defined(POSEMESH_BUILD)
#        define PSM_API __attribute__((visibility("default")))
#    else
#        define PSM_API
#    endif
#elif defined(__EMSCRIPTEN__)
#    define PSM_API
#else
#    error "Platform not supported."
#endif

#endif // __POSEMESH_C_API_H__
