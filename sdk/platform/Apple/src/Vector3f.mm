/* This code is automatically generated from Vector3f.json interface. Do not modify it manually as it will be overwritten! */

#import <Posemesh/Vector3f.h>

#include <Posemesh/Vector3f.hpp>
#include <memory>
#include <new>
#include <utility>

@implementation PSMVector3f {
    std::shared_ptr<psm::Vector3f> m_vector3f;
}

- (instancetype)init
{
    auto* vector3f = new (std::nothrow) psm::Vector3f;
    if (!vector3f) {
        return nil;
    }
    self = [self initWithNativeVector3f:vector3f];
    if (!self) {
        delete vector3f;
        return nil;
    }
    return self;
}

- (instancetype)initWithVector3f:(PSMVector3f*)vector3f
{
    NSAssert(vector3f != nil, @"vector3f is null");
    NSAssert(vector3f->m_vector3f.get() != nullptr, @"vector3f->m_vector3f is null");
    auto* copy = new (std::nothrow) psm::Vector3f(*(vector3f->m_vector3f.get()));
    if (!copy) {
        return nil;
    }
    self = [self initWithNativeVector3f:copy];
    if (!self) {
        delete copy;
        return nil;
    }
    return self;
}

- (instancetype)copyWithZone:(NSZone*)zone
{
    NSAssert(m_vector3f.get() != nullptr, @"m_vector3f is null");
    auto* vector3f = new (std::nothrow) psm::Vector3f(*(m_vector3f.get()));
    if (!vector3f) {
        return nil;
    }
    PSMVector3f* copy = [[[self class] allocWithZone:zone] initWithNativeVector3f:vector3f];
    if (!copy) {
        delete vector3f;
        return nil;
    }
    return copy;
}

- (void)dealloc
{
    NSAssert(m_vector3f.get() != nullptr, @"m_vector3f is null");
}

- (BOOL)isEqual:(id)object
{
    if (self == object) {
        return YES;
    }
    if (![object isKindOfClass:[PSMVector3f class]]) {
        return NO;
    }
    PSMVector3f* vector3f = (PSMVector3f*)object;
    NSAssert(m_vector3f.get() != nullptr, @"m_vector3f is null");
    NSAssert(vector3f->m_vector3f.get() != nullptr, @"vector3f->m_vector3f is null");
    return m_vector3f.get()->operator==(*(vector3f->m_vector3f.get()));
}

- (NSUInteger)hash
{
    NSAssert(m_vector3f.get() != nullptr, @"m_vector3f is null");
    return std::hash<psm::Vector3f> {}(*(m_vector3f.get()));
}

- (float)x
{
    NSAssert(m_vector3f.get() != nullptr, @"m_vector3f is null");
    return m_vector3f.get()->getX();
}

- (void)setX:(float)x
{
    NSAssert(m_vector3f.get() != nullptr, @"m_vector3f is null");
    m_vector3f.get()->setX(x);
}

- (float)y
{
    NSAssert(m_vector3f.get() != nullptr, @"m_vector3f is null");
    return m_vector3f.get()->getY();
}

- (void)setY:(float)y
{
    NSAssert(m_vector3f.get() != nullptr, @"m_vector3f is null");
    m_vector3f.get()->setY(y);
}

- (float)z
{
    NSAssert(m_vector3f.get() != nullptr, @"m_vector3f is null");
    return m_vector3f.get()->getZ();
}

- (void)setZ:(float)z
{
    NSAssert(m_vector3f.get() != nullptr, @"m_vector3f is null");
    m_vector3f.get()->setZ(z);
}

- (instancetype)initWithManagedVector3f:(std::shared_ptr<psm::Vector3f>)vector3f
{
    NSAssert(vector3f.get() != nullptr, @"vector3f is null");
    self = [super init];
    if (!self) {
        return nil;
    }
    m_vector3f = std::move(vector3f);
    return self;
}

- (instancetype)initWithNativeVector3f:(psm::Vector3f*)vector3f
{
    NSAssert(vector3f != nullptr, @"vector3f is null");
    self = [super init];
    if (!self) {
        return nil;
    }
    try {
        m_vector3f.reset(vector3f);
    } catch (...) {
        return nil;
    }
    return self;
}

- (void*)managedVector3f
{
    NSAssert(m_vector3f.get() != nullptr, @"m_vector3f is null");
    return &m_vector3f;
}

- (void*)nativeVector3f
{
    NSAssert(m_vector3f.get() != nullptr, @"m_vector3f is null");
    return m_vector3f.get();
}

@end
