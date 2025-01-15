#ifndef __POSEMESH_POSEMESH_HPP__
#define __POSEMESH_POSEMESH_HPP__

#include <cstdint>
#include <functional>
#include <string>

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

    bool PSM_API sendMessage(
        const void* message,
        std::uint32_t messageSize,
        const std::string& peerId,
        const std::string& protocol,
        std::function<void(bool status)> callback = nullptr
    ) const;

    bool PSM_API sendString(
        const std::string& string,
        bool appendTerminatingNullCharacter,
        const std::string& peerId,
        const std::string& protocol,
        std::function<void(bool status)> callback = nullptr
    ) const;

    void PSM_API pnpSolveDirect(
        const float *objectPoints,
        const float *imagePoints,
        const float *cameraMatrix,
        float *outR,
        float *outT);

    #if !defined(__EMSCRIPTEN__)
        void* __getContext() const noexcept;
    #else
        #if defined(__wasm32__)
            std::uint32_t __getContext() const noexcept;
        #elif defined(__wasm64__)
            std::uint64_t __getContext() const noexcept;
        #else
            #error "Architecture not supported."
        #endif
    #endif

    static std::string PSM_API getVersion();
    static std::string PSM_API getCommitId();
private:
    void* m_context;
};

}

#endif // __POSEMESH_POSEMESH_HPP__
