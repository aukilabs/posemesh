#include <new>
#include <Posemesh/C/Posemesh.h>
#include <Posemesh/Posemesh.hpp>

extern "C" {

psm_posemesh_t* psm_posemesh_create() {
    return new(std::nothrow) psm::Posemesh;
}

void psm_posemesh_destroy(psm_posemesh_t* posemesh) {
    delete posemesh;
}

}
