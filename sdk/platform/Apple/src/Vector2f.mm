/* This code is automatically generated from Vector2f.json interface. Do not modify it manually as it will be overwritten! */

#import <Posemesh/Vector2f.h>

#include <Posemesh/Vector2f.hpp>
#include <memory>
#include <new>
#include <utility>

@implementation PSMVector2f {
    std::shared_ptr<psm::Vector2f> m_vector2f;
}

- (instancetype)init
{
    auto* vector2f = new (std::nothrow) psm::Vector2f;
    if (!vector2f) {
        return nil;
    }
    self = [self initWithNativeVector2f:vector2f];
    if (!self) {
        delete vector2f;
        return nil;
    }
    return self;
}

- (instancetype)initWithVector2f:(PSMVector2f*)vector2f
{
    NSAssert(vector2f != nil, @"vector2f is null");
    NSAssert(vector2f->m_vector2f.get() != nullptr, @"vector2f->m_vector2f is null");
    auto* copy = new (std::nothrow) psm::Vector2f(*(vector2f->m_vector2f.get()));
    if (!copy) {
        return nil;
    }
    self = [self initWithNativeVector2f:copy];
    if (!self) {
        delete copy;
        return nil;
    }
    return self;
}

- (instancetype)copyWithZone:(NSZone*)zone
{
    NSAssert(m_vector2f.get() != nullptr, @"m_vector2f is null");
    auto* vector2f = new (std::nothrow) psm::Vector2f(*(m_vector2f.get()));
    if (!vector2f) {
        return nil;
    }
    PSMVector2f* copy = [[[self class] allocWithZone:zone] initWithNativeVector2f:vector2f];
    if (!copy) {
        delete vector2f;
        return nil;
    }
    return copy;
}

- (void)dealloc
{
    NSAssert(m_vector2f.get() != nullptr, @"m_vector2f is null");
}

- (BOOL)isEqual:(id)object
{
    if (self == object) {
        return YES;
    }
    if (![object isKindOfClass:[PSMVector2f class]]) {
        return NO;
    }
    PSMVector2f* vector2f = (PSMVector2f*)object;
    NSAssert(m_vector2f.get() != nullptr, @"m_vector2f is null");
    NSAssert(vector2f->m_vector2f.get() != nullptr, @"vector2f->m_vector2f is null");
    return m_vector2f.get()->operator==(*(vector2f->m_vector2f.get()));
}

- (NSUInteger)hash
{
    NSAssert(m_vector2f.get() != nullptr, @"m_vector2f is null");
    return std::hash<psm::Vector2f> {}(*(m_vector2f.get()));
}

- (float)x
{
    NSAssert(m_vector2f.get() != nullptr, @"m_vector2f is null");
    return m_vector2f.get()->getX();
}

- (void)setX:(float)x
{
    NSAssert(m_vector2f.get() != nullptr, @"m_vector2f is null");
    m_vector2f.get()->setX(x);
}

- (float)y
{
    NSAssert(m_vector2f.get() != nullptr, @"m_vector2f is null");
    return m_vector2f.get()->getY();
}

- (void)setY:(float)y
{
    NSAssert(m_vector2f.get() != nullptr, @"m_vector2f is null");
    m_vector2f.get()->setY(y);
}

- (float)length
{
    NSAssert(m_vector2f.get() != nullptr, @"m_vector2f is null");
    return m_vector2f.get()->getLength();
}

- (instancetype)initWithManagedVector2f:(std::shared_ptr<psm::Vector2f>)vector2f
{
    NSAssert(vector2f.get() != nullptr, @"vector2f is null");
    self = [super init];
    if (!self) {
        return nil;
    }
    m_vector2f = std::move(vector2f);
    return self;
}

- (instancetype)initWithNativeVector2f:(psm::Vector2f*)vector2f
{
    NSAssert(vector2f != nullptr, @"vector2f is null");
    self = [super init];
    if (!self) {
        return nil;
    }
    try {
        m_vector2f.reset(vector2f);
    } catch (...) {
        return nil;
    }
    return self;
}

- (void*)managedVector2f
{
    NSAssert(m_vector2f.get() != nullptr, @"m_vector2f is null");
    return &m_vector2f;
}

- (void*)nativeVector2f
{
    NSAssert(m_vector2f.get() != nullptr, @"m_vector2f is null");
    return m_vector2f.get();
}

@end
