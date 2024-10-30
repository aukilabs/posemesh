#include <cassert>
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

}
