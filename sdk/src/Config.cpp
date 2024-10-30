#include <Posemesh/Config.hpp>

namespace psm {

Config::Config() {
    #if !defined(__EMSCRIPTEN__)
        m_serveAsBootstrap = false;
        m_serveAsRelay = false;
    #endif
}

Config::Config(const Config& config) = default;

Config::Config(Config&& config) noexcept = default;

Config::~Config() = default;

Config& Config::operator=(const Config& config) = default;

Config& Config::operator=(Config&& config) noexcept = default;

bool Config::operator==(const Config& config) const noexcept {
    if (this == &config)
        return true;
    #if !defined(__EMSCRIPTEN__)
        if(m_serveAsBootstrap != config.m_serveAsBootstrap)
            return false;
        if(m_serveAsRelay != config.m_serveAsRelay)
            return false;
    #endif
    return m_bootstraps == config.m_bootstraps;
}

bool Config::operator!=(const Config& config) const noexcept {
    return !(*this == config);
}

#if !defined(__EMSCRIPTEN__)
    bool Config::getServeAsBootstrap() const noexcept {
        return m_serveAsBootstrap;
    }

    void Config::setServeAsBootstrap(bool serveAsBootstrap) noexcept {
        m_serveAsBootstrap = serveAsBootstrap;
    }

    bool Config::getServeAsRelay() const noexcept {
        return m_serveAsRelay;
    }

    void Config::setServeAsRelay(bool serveAsRelay) noexcept {
        m_serveAsRelay = serveAsRelay;
    }
#endif

}
