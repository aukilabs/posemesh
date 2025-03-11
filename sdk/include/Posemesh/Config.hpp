#ifndef __POSEMESH_CONFIG_HPP__
#define __POSEMESH_CONFIG_HPP__

#include <cstdint>
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

    std::vector<std::string> PSM_API getBootstraps() const;
    bool PSM_API setBootstraps(std::vector<std::string> bootstraps) noexcept;

    std::vector<std::string> PSM_API getRelays() const;
    bool PSM_API setRelays(std::vector<std::string> relays) noexcept;

    std::vector<std::uint8_t> PSM_API getPrivateKey() const;
    void PSM_API setPrivateKey(std::vector<std::uint8_t> privateKey) noexcept;

#if !defined(__EMSCRIPTEN__)
    std::string PSM_API getPrivateKeyPath() const;
    void PSM_API setPrivateKeyPath(std::string privateKeyPath) noexcept;

    bool PSM_API getEnableMDNS() const noexcept;
    void PSM_API setEnableMDNS(bool enableMDNS) noexcept;
#endif

    std::string PSM_API getName() const;
    void PSM_API setName(std::string name) noexcept;

    static Config PSM_API createDefault();

private:
    std::vector<std::string> m_bootstraps;
    std::vector<std::string> m_relays;
    std::vector<std::uint8_t> m_privateKey;
#if !defined(__EMSCRIPTEN__)
    std::string m_privateKeyPath;
    bool m_enableMDNS;
#endif
    std::string m_name;
};

}

#endif // __POSEMESH_CONFIG_HPP__
