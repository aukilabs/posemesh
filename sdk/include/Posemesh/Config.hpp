#ifndef __POSEMESH_CONFIG_HPP__
#define __POSEMESH_CONFIG_HPP__

#include <string>
#include <vector>

#include "API.hpp"

namespace psm {

class Config final {
public:
    PSM_API Config();
    PSM_API Config(const Config& config);
    PSM_API Config(Config&& config) noexcept;
    PSM_API ~Config();

    Config& PSM_API operator=(const Config& config);
    Config& PSM_API operator=(Config&& config) noexcept;

    bool PSM_API operator==(const Config& config) const noexcept;
    bool PSM_API operator!=(const Config& config) const noexcept;

    #if !defined(__EMSCRIPTEN__)
        bool PSM_API getServeAsBootstrap() const noexcept;
        void PSM_API setServeAsBootstrap(bool serveAsBootstrap) noexcept;

        bool PSM_API getServeAsRelay() const noexcept;
        void PSM_API setServeAsRelay(bool serveAsRelay) noexcept;
    #endif

    std::vector<std::string> PSM_API getBootstraps() const;
    bool PSM_API setBootstraps(std::vector<std::string> bootstraps) noexcept;

    std::vector<std::string> PSM_API getRelays() const;
    bool PSM_API setRelays(std::vector<std::string> relays) noexcept;

    static Config PSM_API createDefault();
private:
    #if !defined(__EMSCRIPTEN__)
        bool m_serveAsBootstrap;
        bool m_serveAsRelay;
    #endif
    std::vector<std::string> m_bootstraps;
    std::vector<std::string> m_relays;
};

}

#endif // __POSEMESH_CONFIG_HPP__
