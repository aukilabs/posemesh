#import <Posemesh/Posemesh.h>

#include <new>
#include <Posemesh/Posemesh.hpp>

@implementation PSMPosemesh {
    psm::Posemesh* m_posemesh;
}

- (instancetype)init {
    auto* posemesh = new(std::nothrow) psm::Posemesh;
    if (!posemesh) {
        return nil;
    }
    self = [self initWithNativePosemesh:posemesh];
    if (!self) {
        delete posemesh;
        return nil;
    }
    return self;
}

- (instancetype)initWithConfig:(PSMConfig*)config {
    NSAssert(config, @"config is null");
    NSAssert([config nativeConfig], @"[config nativeConfig] is null");
    auto* posemesh = new(std::nothrow) psm::Posemesh(*static_cast<psm::Config*>([config nativeConfig]));
    if (!posemesh) {
        return nil;
    }
    self = [self initWithNativePosemesh:posemesh];
    if (!self) {
        delete posemesh;
        return nil;
    }
    return self;
}

- (instancetype)initWithNativePosemesh:(psm::Posemesh*)posemesh {
    NSAssert(posemesh, @"posemesh is null");
    self = [super init];
    if (!self) {
        return nil;
    }
    m_posemesh = posemesh;
    return self;
}

- (void)dealloc {
    NSAssert(m_posemesh, @"m_posemesh is null");
    delete m_posemesh;
    m_posemesh = nullptr;
}

- (BOOL)isEqual:(id)object {
    if (self == object)
        return YES;
    if (![object isKindOfClass:[PSMPosemesh class]])
        return NO;
    PSMPosemesh* posemesh = (PSMPosemesh*)object;
    NSAssert(m_posemesh, @"m_posemesh is null");
    NSAssert(posemesh->m_posemesh, @"posemesh->m_posemesh is null");
    return m_posemesh == posemesh->m_posemesh;
}

- (NSUInteger)hash {
    NSAssert(m_posemesh, @"m_posemesh is null");
    return reinterpret_cast<NSUInteger>(m_posemesh);
}

- (void*)nativePosemesh {
    NSAssert(m_posemesh, @"m_posemesh is null");
    return m_posemesh;
}

@end
