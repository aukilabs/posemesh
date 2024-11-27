#include <cassert>
#include <Posemesh/Networking/API.h>
#include <Posemesh/Posemesh.hpp>
#include <sstream>

namespace psm {

Posemesh::Posemesh() : Posemesh(Config::createDefault()) { }

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
    std::string relays;
    {
        std::ostringstream stream;
        bool first = true;
        for (const auto& relay : config.getRelays()) {
            if (first)
                first = false;
            else
                stream << ';';
            stream << relay;
        }
        relays = stream.str();
    }
    const psm_posemesh_networking_config_t nativeConfig {
        #if !defined(__EMSCRIPTEN__)
            .serve_as_bootstrap = static_cast<uint8_t>(config.getServeAsBootstrap()),
            .serve_as_relay = static_cast<uint8_t>(config.getServeAsRelay()),
        #endif
        .bootstraps = bootstraps.c_str(),
        .relays = relays.c_str()
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

bool Posemesh::sendMessage(
    const void* message,
    std::uint32_t messageSize,
    const std::string& peerId,
    const std::string& protocol,
    std::function<void(bool status)> callback
) const {
    assert(m_context || !"Posemesh::sendMessage(): m_context is null");
    assert(message || !"Posemesh::sendMessage(): message is null");
    assert(messageSize != 0 || !"Posemesh::sendMessage(): messageSize is zero");
    assert(!peerId.empty() || !"Posemesh::sendMessage(): peerId is empty");
    assert(!protocol.empty() || !"Posemesh::sendMessage(): protocol is empty");
    auto wrappedCallback = callback ? std::make_unique<std::function<void(bool status)>>(std::move(callback)) : std::unique_ptr<std::function<void(bool status)>>{};
    const auto result = static_cast<bool>(psm_posemesh_networking_context_send_message(
        static_cast<psm_posemesh_networking_context_t*>(m_context),
        message,
        messageSize,
        peerId.c_str(),
        protocol.c_str(),
        wrappedCallback.get(),
        wrappedCallback ? [](std::uint8_t status, void* userData) -> void {
            const std::unique_ptr<std::function<void(bool status)>> wrappedCallback(static_cast<std::function<void(bool status)>*>(userData));
            assert(wrappedCallback);
            const auto& callback = *wrappedCallback;
            assert(callback);
            callback(static_cast<bool>(status));
        } : nullptr
    ));
    if (result)
        wrappedCallback.release();
    return result;
}

bool Posemesh::sendString(
    const std::string& string,
    bool appendTerminatingNullCharacter,
    const std::string& peerId,
    const std::string& protocol,
    std::function<void(bool status)> callback
) const {
    return sendMessage(
        string.c_str(),
        string.size() + (appendTerminatingNullCharacter ? 1 : 0),
        peerId,
        protocol,
        callback
    );
}

#if !defined(__EMSCRIPTEN__)
    void* Posemesh::__getContext() const noexcept {
        return m_context;
    }
#else
    #if defined(__wasm32__)
        std::uint32_t Posemesh::__getContext() const noexcept {
            return reinterpret_cast<std::uint32_t>(m_context);
        }
    #elif defined(__wasm64__)
        std::uint64_t Posemesh::__getContext() const noexcept {
            return reinterpret_cast<std::uint64_t>(m_context);
        }
    #else
        #error "Architecture not supported."
    #endif
#endif

std::string Posemesh::getVersion() {
    return POSEMESH_VERSION;
}

std::string Posemesh::getCommitId() {
    return POSEMESH_COMMIT_ID;
}

}
