#include <cassert>
#include <cstring>
#include <limits>
#include <new>
#include <Posemesh/C/Config.h>
#include <Posemesh/Config.hpp>

extern "C" {

psm_config_t* psm_config_create() {
    return new(std::nothrow) psm::Config;
}

psm_config_t* psm_config_duplicate(const psm_config_t* config) {
    if (!config) {
        assert(!"psm_config_duplicate(): config is null");
        return nullptr;
    }
    return new(std::nothrow) psm::Config(*config);
}

uint8_t PSM_API psm_config_equals(const psm_config_t* config, const psm_config_t* other_config) {
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

void psm_config_destroy(psm_config_t* config) {
    delete config;
}

#if !defined(__EMSCRIPTEN__)
    uint8_t psm_config_get_serve_as_bootstrap(const psm_config_t* config) {
        if (!config) {
            assert(!"psm_config_get_serve_as_bootstrap(): config is null");
            return 0;
        }
        return static_cast<uint8_t>(config->getServeAsBootstrap());
    }

    void psm_config_set_serve_as_bootstrap(psm_config_t* config, uint8_t serve_as_bootstrap) {
        if (!config) {
            assert(!"psm_config_set_serve_as_bootstrap(): config is null");
            return;
        }
        config->setServeAsBootstrap(static_cast<bool>(serve_as_bootstrap));
    }

    uint8_t psm_config_get_serve_as_relay(const psm_config_t* config) {
        if (!config) {
            assert(!"psm_config_get_serve_as_relay(): config is null");
            return 0;
        }
        return static_cast<uint8_t>(config->getServeAsRelay());
    }

    void psm_config_set_serve_as_relay(psm_config_t* config, uint8_t serve_as_relay) {
        if (!config) {
            assert(!"psm_config_set_serve_as_relay(): config is null");
            return;
        }
        config->setServeAsRelay(static_cast<bool>(serve_as_relay));
    }
#endif

const char* const* psm_config_get_bootstraps(const psm_config_t* config, uint32_t* out_bootstraps_count) {
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
    char* buffer = new(std::nothrow) char[buffer_size];
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

void psm_config_get_bootstraps_free(const char* const* bootstraps) {
    delete[] const_cast<char*>(reinterpret_cast<const char*>(bootstraps));
}

uint8_t psm_config_set_bootstraps(psm_config_t* config, const char* const* bootstraps, uint32_t bootstraps_count) {
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

}
