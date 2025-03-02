/* This code is automatically generated from Vector4f.json interface. Do not modify it manually as it will be overwritten! */

#import <Posemesh/Vector4f.h>

#include <Posemesh/Vector4f.hpp>
#include <memory>
#include <new>
#include <utility>

@implementation PSMVector4f {
    std::shared_ptr<psm::Vector4f> m_vector4f;
}

- (instancetype)init
{
    auto* vector4f = new (std::nothrow) psm::Vector4f;
    if (!vector4f) {
        return nil;
    }
    self = [self initWithNativeVector4f:vector4f];
    if (!self) {
        delete vector4f;
        return nil;
    }
    return self;
}

- (instancetype)initWithVector4f:(PSMVector4f*)vector4f
{
    NSAssert(vector4f != nil, @"vector4f is null");
    NSAssert(vector4f->m_vector4f.get() != nullptr, @"vector4f->m_vector4f is null");
    auto* copy = new (std::nothrow) psm::Vector4f(*(vector4f->m_vector4f.get()));
    if (!copy) {
        return nil;
    }
    self = [self initWithNativeVector4f:copy];
    if (!self) {
        delete copy;
        return nil;
    }
    return self;
}

- (instancetype)copyWithZone:(NSZone*)zone
{
    NSAssert(m_vector4f.get() != nullptr, @"m_vector4f is null");
    auto* vector4f = new (std::nothrow) psm::Vector4f(*(m_vector4f.get()));
    if (!vector4f) {
        return nil;
    }
    PSMVector4f* copy = [[[self class] allocWithZone:zone] initWithNativeVector4f:vector4f];
    if (!copy) {
        delete vector4f;
        return nil;
    }
    return copy;
}

- (void)dealloc
{
    NSAssert(m_vector4f.get() != nullptr, @"m_vector4f is null");
}

- (BOOL)isEqual:(id)object
{
    if (self == object) {
        return YES;
    }
    if (![object isKindOfClass:[PSMVector4f class]]) {
        return NO;
    }
    PSMVector4f* vector4f = (PSMVector4f*)object;
    NSAssert(m_vector4f.get() != nullptr, @"m_vector4f is null");
    NSAssert(vector4f->m_vector4f.get() != nullptr, @"vector4f->m_vector4f is null");
    return m_vector4f.get()->operator==(*(vector4f->m_vector4f.get()));
}

- (NSUInteger)hash
{
    NSAssert(m_vector4f.get() != nullptr, @"m_vector4f is null");
    return std::hash<psm::Vector4f> {}(*(m_vector4f.get()));
}

- (float)x
{
    NSAssert(m_vector4f.get() != nullptr, @"m_vector4f is null");
    return m_vector4f.get()->getX();
}

- (void)setX:(float)x
{
    NSAssert(m_vector4f.get() != nullptr, @"m_vector4f is null");
    m_vector4f.get()->setX(x);
}

- (float)y
{
    NSAssert(m_vector4f.get() != nullptr, @"m_vector4f is null");
    return m_vector4f.get()->getY();
}

- (void)setY:(float)y
{
    NSAssert(m_vector4f.get() != nullptr, @"m_vector4f is null");
    m_vector4f.get()->setY(y);
}

- (float)z
{
    NSAssert(m_vector4f.get() != nullptr, @"m_vector4f is null");
    return m_vector4f.get()->getZ();
}

- (void)setZ:(float)z
{
    NSAssert(m_vector4f.get() != nullptr, @"m_vector4f is null");
    m_vector4f.get()->setZ(z);
}

- (float)w
{
    NSAssert(m_vector4f.get() != nullptr, @"m_vector4f is null");
    return m_vector4f.get()->getW();
}

- (void)setW:(float)w
{
    NSAssert(m_vector4f.get() != nullptr, @"m_vector4f is null");
    m_vector4f.get()->setW(w);
}

- (instancetype)initWithManagedVector4f:(std::shared_ptr<psm::Vector4f>)vector4f
{
    NSAssert(vector4f.get() != nullptr, @"vector4f is null");
    self = [super init];
    if (!self) {
        return nil;
    }
    m_vector4f = std::move(vector4f);
    return self;
}

- (instancetype)initWithNativeVector4f:(psm::Vector4f*)vector4f
{
    NSAssert(vector4f != nullptr, @"vector4f is null");
    self = [super init];
    if (!self) {
        return nil;
    }
    try {
        m_vector4f.reset(vector4f);
    } catch (...) {
        return nil;
    }
    return self;
}

- (void*)managedVector4f
{
    NSAssert(m_vector4f.get() != nullptr, @"m_vector4f is null");
    return &m_vector4f;
}

- (void*)nativeVector4f
{
    NSAssert(m_vector4f.get() != nullptr, @"m_vector4f is null");
    return m_vector4f.get();
}

@end
