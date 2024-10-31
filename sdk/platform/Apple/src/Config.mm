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

- (instancetype)initWithConfig:(PSMConfig*)config {
    NSAssert(config, @"config is null");
    NSAssert(config->m_config, @"config->m_config is null");
    auto* copy = new(std::nothrow) psm::Config(*(config->m_config));
    if (!copy) {
        return nil;
    }
    self = [self initWithNativeConfig:copy];
    if (!self) {
        delete copy;
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

- (BOOL)isEqual:(id)object {
    if (self == object)
        return YES;
    if (![object isKindOfClass:[PSMConfig class]])
        return NO;
    PSMConfig* config = (PSMConfig*)object;
    NSAssert(m_config, @"m_config is null");
    NSAssert(config->m_config, @"config->m_config is null");
    return m_config->operator==(*(config->m_config));
}

- (NSUInteger)hash {
    return 0;
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

- (NSArray<NSString*>*)bootstraps {
    NSAssert(m_config, @"m_config is null");
    const auto bootstraps = m_config->getBootstraps();
    NSMutableArray<NSString*>* array = [[NSMutableArray<NSString*> alloc] init];
    for (const auto& bootstrap : bootstraps) {
        [array addObject:[NSString stringWithUTF8String:bootstrap.c_str()]];
    }
    return array;
}

- (BOOL)setBootstraps:(NSArray<NSString*>*)bootstraps {
    NSAssert(m_config, @"m_config is null");
    std::vector<std::string> bootstraps_vector;
    for (NSString* bootstrap in bootstraps) {
        bootstraps_vector.emplace_back([bootstrap UTF8String]);
    }
    return static_cast<BOOL>(m_config->setBootstraps(std::move(bootstraps_vector)));
}

- (void*)nativeConfig {
    NSAssert(m_config, @"m_config is null");
    return m_config;
}

@end
