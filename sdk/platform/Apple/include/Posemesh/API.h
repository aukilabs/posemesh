#if defined(POSEMESH_BUILD)
#    define PSM_API __attribute__((visibility("default")))
#else
#    define PSM_API
#endif
