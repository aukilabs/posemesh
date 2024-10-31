#include <cassert>
#include <new>
#include <Posemesh/C/Posemesh.h>
#include <Posemesh/Posemesh.hpp>

extern "C" {

psm_posemesh_t* psm_posemesh_create() {
    return new(std::nothrow) psm::Posemesh;
}

psm_posemesh_t* psm_posemesh_create_with_config(const psm_config_t* config) {
    if (!config) {
        assert(!"psm_posemesh_create_with_config(): config is null");
        return nullptr;
    }
    return new(std::nothrow) psm::Posemesh(*config);
}

void psm_posemesh_destroy(psm_posemesh_t* posemesh) {
    delete posemesh;
}

}
