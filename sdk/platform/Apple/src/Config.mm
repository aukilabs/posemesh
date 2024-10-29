#import <Posemesh/Config.h>

#include <new>
#include <Posemesh/Config.hpp>

@implementation PSMConfig {
    psm::Config* m_config;
}

- (instancetype)init {
    auto* config = new(std::nothrow) psm::Config;
    if (!config) {
        return nil;
    }
    self = [self initWithNativeConfig:config];
    if (!self) {
        delete config;
        return nil;
    }
    return self;
}

- (instancetype)initWithNativeConfig:(psm::Config*)config {
    NSAssert(config, @"config is null");
    self = [super init];
    if (!self) {
        return nil;
    }
    m_config = config;
    return self;
}

- (instancetype)copyWithZone:(NSZone*)zone {
    NSAssert(m_config, @"m_config is null");
    auto* config = new(std::nothrow) psm::Config(*m_config);
    if (!config) {
        return nil;
    }
    PSMConfig* copy = [[[self class] allocWithZone:zone] initWithNativeConfig:config];
    if (!copy) {
        delete config;
        return nil;
    }
    return copy;
}

- (void)dealloc {
    NSAssert(m_config, @"m_config is null");
    delete m_config;
    m_config = nullptr;
}

- (BOOL)serveAsBootstrap {
    NSAssert(m_config, @"m_config is null");
    return static_cast<BOOL>(m_config->getServeAsBootstrap());
}

- (void)setServeAsBootstrap:(BOOL)serveAsBootstrap {
    NSAssert(m_config, @"m_config is null");
    m_config->setServeAsBootstrap(static_cast<bool>(serveAsBootstrap));
}

- (BOOL)serveAsRelay {
    NSAssert(m_config, @"m_config is null");
    return static_cast<BOOL>(m_config->getServeAsRelay());
}

- (void)setServeAsRelay:(BOOL)serveAsRelay {
    NSAssert(m_config, @"m_config is null");
    m_config->setServeAsRelay(static_cast<bool>(serveAsRelay));
}

@end
