#ifndef __POSEMESH_POSEMESH_HPP__
#define __POSEMESH_POSEMESH_HPP__

#include <cstdint>

#include "API.hpp"
#include "Config.hpp"

namespace psm {

class Posemesh final {
public:
    PSM_API Posemesh();
    PSM_API Posemesh(const Config& config);
    Posemesh(const Posemesh& posemesh) = delete;
    PSM_API Posemesh(Posemesh&& posemesh);
    PSM_API ~Posemesh();

    Posemesh& operator=(const Posemesh& posemesh) = delete;
    Posemesh& PSM_API operator=(Posemesh&& posemesh) noexcept;

    #if defined(__EMSCRIPTEN__)
        #if defined(__wasm32__)
            std::uint32_t __getContext() const noexcept;
        #elif defined(__wasm64__)
            std::uint64_t __getContext() const noexcept;
        #else
            #error "Architecture not supported."
        #endif
    #endif
private:
    void* m_context;
};

}

#endif // __POSEMESH_POSEMESH_HPP__
