#include <Posemesh/Config.hpp>

namespace psm {

Config::Config() = default;

Config::Config(const Config& config) = default;

Config::Config(Config&& config) noexcept = default;

Config::~Config() = default;

Config& Config::operator=(const Config& config) = default;

Config& Config::operator=(Config&& config) noexcept = default;

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
