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

@end
