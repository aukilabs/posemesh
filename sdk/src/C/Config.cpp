#include <Posemesh/C/Config.h>
#include <Posemesh/Config.hpp>
#include <cassert>
#include <cstring>
#include <limits>
#include <new>

extern "C" {

psm_config_t* psm_config_create()
{
    return new (std::nothrow) psm::Config;
}

psm_config_t* psm_config_create_default()
{
    return new (std::nothrow) psm::Config(std::move(psm::Config::createDefault()));
}

psm_config_t* psm_config_duplicate(const psm_config_t* config)
{
    if (!config) {
        assert(!"psm_config_duplicate(): config is null");
        return nullptr;
    }
    return new (std::nothrow) psm::Config(*config);
}

uint8_t PSM_API psm_config_equals(const psm_config_t* config, const psm_config_t* other_config)
{
    if (!config) {
        assert(!"psm_config_equals(): config is null");
        return 0;
    }
    if (!other_config) {
        assert(!"psm_config_equals(): other_config is null");
        return 0;
    }
    return static_cast<uint8_t>(config->operator==(*other_config));
}

void psm_config_destroy(psm_config_t* config)
{
    delete config;
}

const char* const* psm_config_get_bootstraps(const psm_config_t* config, uint32_t* out_bootstraps_count)
{
    if (!config) {
        assert(!"psm_config_get_bootstraps(): config is null");
        return nullptr;
    }
    const auto bootstraps = config->getBootstraps();
    if (bootstraps.size() > std::numeric_limits<uint32_t>::max()) {
        assert(!"psm_config_get_bootstraps(): bootstraps count overflow");
        return nullptr;
    }
    const auto bootstraps_count = static_cast<uint32_t>(bootstraps.size());
    auto buffer_size = (bootstraps_count + 1) * sizeof(char*);
    const auto prefix_offset = buffer_size;
    for (const auto& bootstrap : bootstraps) {
        buffer_size += bootstrap.size() + 1;
    }
    char* buffer = new (std::nothrow) char[buffer_size];
    char** prefix_ptr = reinterpret_cast<char**>(buffer);
    char* content_ptr = buffer + prefix_offset;
    for (const auto& bootstrap : bootstraps) {
        *prefix_ptr = content_ptr;
        prefix_ptr++;
        std::memcpy(content_ptr, bootstrap.data(), bootstrap.size() + 1);
        content_ptr += bootstrap.size() + 1;
    }
    *prefix_ptr = nullptr;
    if (out_bootstraps_count) {
        *out_bootstraps_count = bootstraps_count;
    }
    return reinterpret_cast<const char* const*>(buffer);
}

void psm_config_get_bootstraps_free(const char* const* bootstraps)
{
    delete[] const_cast<char*>(reinterpret_cast<const char*>(bootstraps));
}

uint8_t psm_config_set_bootstraps(psm_config_t* config, const char* const* bootstraps, uint32_t bootstraps_count)
{
    if (!config) {
        assert(!"psm_config_set_bootstraps(): config is null");
        return 0;
    }
    std::vector<std::string> bootstraps_vector;
    if (bootstraps) {
        if (bootstraps_count <= 0) {
            bootstraps_count = 0;
            const auto* const* bootstraps_ptr = bootstraps;
            while (*bootstraps_ptr) {
                bootstraps_count++;
                bootstraps_ptr++;
            }
        }
        for (uint32_t i = 0; i < bootstraps_count; ++i) {
            const char* bootstrap = bootstraps[i];
            if (!bootstrap) {
                assert(!"psm_config_set_bootstraps(): at least one of the bootstraps is null");
                return 0;
            }
            bootstraps_vector.emplace_back(bootstrap);
        }
    }
    return static_cast<uint8_t>(config->setBootstraps(std::move(bootstraps_vector)));
}

const char* const* psm_config_get_relays(const psm_config_t* config, uint32_t* out_relays_count)
{
    if (!config) {
        assert(!"psm_config_get_relays(): config is null");
        return nullptr;
    }
    const auto relays = config->getRelays();
    if (relays.size() > std::numeric_limits<uint32_t>::max()) {
        assert(!"psm_config_get_relays(): relays count overflow");
        return nullptr;
    }
    const auto relays_count = static_cast<uint32_t>(relays.size());
    auto buffer_size = (relays_count + 1) * sizeof(char*);
    const auto prefix_offset = buffer_size;
    for (const auto& relay : relays) {
        buffer_size += relay.size() + 1;
    }
    char* buffer = new (std::nothrow) char[buffer_size];
    char** prefix_ptr = reinterpret_cast<char**>(buffer);
    char* content_ptr = buffer + prefix_offset;
    for (const auto& relay : relays) {
        *prefix_ptr = content_ptr;
        prefix_ptr++;
        std::memcpy(content_ptr, relay.data(), relay.size() + 1);
        content_ptr += relay.size() + 1;
    }
    *prefix_ptr = nullptr;
    if (out_relays_count) {
        *out_relays_count = relays_count;
    }
    return reinterpret_cast<const char* const*>(buffer);
}

void psm_config_get_relays_free(const char* const* relays)
{
    delete[] const_cast<char*>(reinterpret_cast<const char*>(relays));
}

uint8_t psm_config_set_relays(psm_config_t* config, const char* const* relays, uint32_t relays_count)
{
    if (!config) {
        assert(!"psm_config_set_relays(): config is null");
        return 0;
    }
    std::vector<std::string> relays_vector;
    if (relays) {
        if (relays_count <= 0) {
            relays_count = 0;
            const auto* const* relays_ptr = relays;
            while (*relays_ptr) {
                relays_count++;
                relays_ptr++;
            }
        }
        for (uint32_t i = 0; i < relays_count; ++i) {
            const char* relay = relays[i];
            if (!relay) {
                assert(!"psm_config_set_relays(): at least one of the relays is null");
                return 0;
            }
            relays_vector.emplace_back(relay);
        }
    }
    return static_cast<uint8_t>(config->setRelays(std::move(relays_vector)));
}

const uint8_t* psm_config_get_private_key(const psm_config_t* config, uint32_t* out_private_key_size)
{
    if (!config) {
        assert(!"psm_config_get_private_key(): config is null");
        if (out_private_key_size) {
            *out_private_key_size = 0;
        }
        return nullptr;
    }

    const auto private_key = config->getPrivateKey();
    if (private_key.empty()) {
        if (out_private_key_size) {
            *out_private_key_size = 0;
        }
        return nullptr;
    }

    auto* result = new (std::nothrow) std::uint8_t[private_key.size()];
    std::memcpy(result, private_key.data(), private_key.size());
    if (out_private_key_size) {
        *out_private_key_size = static_cast<std::uint32_t>(private_key.size());
    }
    return result;
}

void psm_config_get_private_key_free(const uint8_t* private_key)
{
    delete[] const_cast<uint8_t*>(private_key);
}

void psm_config_set_private_key(psm_config_t* config, const uint8_t* private_key, uint32_t private_key_size)
{
    if (!config) {
        assert(!"psm_config_set_private_key(): config is null");
        return;
    }
    if (!private_key && private_key_size != 0) {
        assert(!"psm_config_set_private_key(): private_key is null and private_key_size is non-zero");
        return;
    }
    config->setPrivateKey(std::vector<std::uint8_t> { private_key, private_key + private_key_size });
}

#if !defined(__EMSCRIPTEN__)
const char* PSM_API psm_config_get_private_key_path(const psm_config_t* config)
{
    if (!config) {
        assert(!"psm_config_get_private_key_path(): config is null");
        return nullptr;
    }

    const auto private_key_path = config->getPrivateKeyPath();
    if (private_key_path.empty()) {
        return nullptr;
    }

    auto* result = new (std::nothrow) char[private_key_path.size() + 1];
    std::memcpy(result, private_key_path.c_str(), private_key_path.size() + 1);
    return result;
}

void psm_config_get_private_key_path_free(const char* private_key_path)
{
    delete[] const_cast<char*>(private_key_path);
}

void PSM_API psm_config_set_private_key_path(psm_config_t* config, const char* private_key_path)
{
    if (!config) {
        assert(!"psm_config_set_private_key_path(): config is null");
        return;
    }
    config->setPrivateKeyPath(private_key_path ? std::string { private_key_path } : std::string {});
}

uint8_t psm_config_get_enable_mdns(const psm_config_t* config)
{
    if (!config) {
        assert(!"psm_config_get_enable_mdns(): config is null");
        return 0;
    }
    return static_cast<uint8_t>(config->getEnableMDNS());
}

void psm_config_set_enable_mdns(psm_config_t* config, uint8_t enable_mdns)
{
    if (!config) {
        assert(!"psm_config_set_enable_mdns(): config is null");
        return;
    }
    config->setEnableMDNS(static_cast<bool>(enable_mdns));
}
#endif

const char* PSM_API psm_config_get_name(const psm_config_t* config)
{
    if (!config) {
        assert(!"psm_config_get_name(): config is null");
        return nullptr;
    }

    const auto name = config->getName();
    if (name.empty()) {
        return nullptr;
    }

    auto* result = new (std::nothrow) char[name.size() + 1];
    std::memcpy(result, name.c_str(), name.size() + 1);
    return result;
}

void psm_config_get_name_free(const char* name)
{
    delete[] const_cast<char*>(name);
}

void PSM_API psm_config_set_name(psm_config_t* config, const char* name)
{
    if (!config) {
        assert(!"psm_config_set_name(): config is null");
        return;
    }
    config->setName(name ? std::string { name } : std::string {});
}
}
