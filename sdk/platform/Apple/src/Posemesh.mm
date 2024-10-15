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
    self = [super init];
    if (!self) {
        delete posemesh;
        return nil;
    }
    m_posemesh = posemesh;
    return self;
}

- (void)dealloc {
    delete m_posemesh;
}

@end
