#include <cassert>
#include <Posemesh/Networking/API.h>
#include <Posemesh/Posemesh.hpp>
#include <sstream>

namespace psm {

Posemesh::Posemesh() : Posemesh(Config { }) { } // TODO: impl with default settings

Posemesh::Posemesh(const Config& config) {
    std::string bootstraps;
    {
        std::ostringstream stream;
        bool first = true;
        for (const auto& bootstrap : config.getBootstraps()) {
            if (first)
                first = false;
            else
                stream << ';';
            stream << bootstrap;
        }
        bootstraps = stream.str();
    }
    const psm_posemesh_networking_config_t nativeConfig {
        #if !defined(__EMSCRIPTEN__)
            .serve_as_bootstrap = static_cast<uint8_t>(config.getServeAsBootstrap()),
            .serve_as_relay = static_cast<uint8_t>(config.getServeAsRelay()),
        #endif
        .bootstraps = bootstraps.c_str()
    };
    m_context = psm_posemesh_networking_context_create(&nativeConfig);
    assert(m_context || !"Posemesh::Posemesh(): failed to create the Posemesh Networking context");
}

Posemesh::Posemesh(Posemesh&& posemesh) {
    assert(posemesh.m_context || !"Posemesh::Posemesh(): posemesh.m_context is null");
    if (m_context)
        psm_posemesh_networking_context_destroy(static_cast<psm_posemesh_networking_context_t*>(m_context));
    m_context = posemesh.m_context;
    posemesh.m_context = nullptr;
}

Posemesh::~Posemesh() {
    assert(m_context || !"Posemesh::~Posemesh(): m_context is null");
    psm_posemesh_networking_context_destroy(static_cast<psm_posemesh_networking_context_t*>(m_context));
    m_context = nullptr;
}

Posemesh& Posemesh::operator=(Posemesh&& posemesh) noexcept {
    assert(posemesh.m_context || !"Posemesh::operator=(): posemesh.m_context is null");
    if (this == &posemesh)
        return *this;
    if (m_context)
        psm_posemesh_networking_context_destroy(static_cast<psm_posemesh_networking_context_t*>(m_context));
    m_context = posemesh.m_context;
    posemesh.m_context = nullptr;
    return *this;
}

}
