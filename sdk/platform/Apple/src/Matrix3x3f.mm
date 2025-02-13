/* This code is automatically generated from Matrix3x3f.json interface. Do not modify it manually as it will be overwritten! */

#import <Posemesh/Matrix3x3f.h>

#include <Posemesh/Matrix3x3f.hpp>
#include <memory>
#include <new>
#include <utility>

@implementation PSMMatrix3x3f {
    std::shared_ptr<psm::Matrix3x3f> m_matrix3x3f;
}

- (instancetype)init
{
    auto* matrix3x3f = new (std::nothrow) psm::Matrix3x3f;
    if (!matrix3x3f) {
        return nil;
    }
    self = [self initWithNativeMatrix3x3f:matrix3x3f];
    if (!self) {
        delete matrix3x3f;
        return nil;
    }
    return self;
}

- (instancetype)initWithMatrix3x3f:(PSMMatrix3x3f*)matrix3x3f
{
    NSAssert(matrix3x3f != nil, @"matrix3x3f is null");
    NSAssert(matrix3x3f->m_matrix3x3f.get() != nullptr, @"matrix3x3f->m_matrix3x3f is null");
    auto* copy = new (std::nothrow) psm::Matrix3x3f(*(matrix3x3f->m_matrix3x3f.get()));
    if (!copy) {
        return nil;
    }
    self = [self initWithNativeMatrix3x3f:copy];
    if (!self) {
        delete copy;
        return nil;
    }
    return self;
}

- (instancetype)copyWithZone:(NSZone*)zone
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    auto* matrix3x3f = new (std::nothrow) psm::Matrix3x3f(*(m_matrix3x3f.get()));
    if (!matrix3x3f) {
        return nil;
    }
    PSMMatrix3x3f* copy = [[[self class] allocWithZone:zone] initWithNativeMatrix3x3f:matrix3x3f];
    if (!copy) {
        delete matrix3x3f;
        return nil;
    }
    return copy;
}

- (void)dealloc
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
}

- (BOOL)isEqual:(id)object
{
    if (self == object) {
        return YES;
    }
    if (![object isKindOfClass:[PSMMatrix3x3f class]]) {
        return NO;
    }
    PSMMatrix3x3f* matrix3x3f = (PSMMatrix3x3f*)object;
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    NSAssert(matrix3x3f->m_matrix3x3f.get() != nullptr, @"matrix3x3f->m_matrix3x3f is null");
    return m_matrix3x3f.get()->operator==(*(matrix3x3f->m_matrix3x3f.get()));
}

- (NSUInteger)hash
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    return std::hash<psm::Matrix3x3f> {}(*(m_matrix3x3f.get()));
}

- (float)m00
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    return m_matrix3x3f.get()->getM00();
}

- (void)setM00:(float)m00
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    m_matrix3x3f.get()->setM00(m00);
}

- (float)m01
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    return m_matrix3x3f.get()->getM01();
}

- (void)setM01:(float)m01
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    m_matrix3x3f.get()->setM01(m01);
}

- (float)m02
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    return m_matrix3x3f.get()->getM02();
}

- (void)setM02:(float)m02
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    m_matrix3x3f.get()->setM02(m02);
}

- (float)m03
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    return m_matrix3x3f.get()->getM03();
}

- (void)setM03:(float)m03
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    m_matrix3x3f.get()->setM03(m03);
}

- (float)m10
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    return m_matrix3x3f.get()->getM10();
}

- (void)setM10:(float)m10
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    m_matrix3x3f.get()->setM10(m10);
}

- (float)m11
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    return m_matrix3x3f.get()->getM11();
}

- (void)setM11:(float)m11
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    m_matrix3x3f.get()->setM11(m11);
}

- (float)m12
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    return m_matrix3x3f.get()->getM12();
}

- (void)setM12:(float)m12
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    m_matrix3x3f.get()->setM12(m12);
}

- (float)m13
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    return m_matrix3x3f.get()->getM13();
}

- (void)setM13:(float)m13
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    m_matrix3x3f.get()->setM13(m13);
}

- (float)m20
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    return m_matrix3x3f.get()->getM20();
}

- (void)setM20:(float)m20
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    m_matrix3x3f.get()->setM20(m20);
}

- (float)m21
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    return m_matrix3x3f.get()->getM21();
}

- (void)setM21:(float)m21
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    m_matrix3x3f.get()->setM21(m21);
}

- (float)m22
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    return m_matrix3x3f.get()->getM22();
}

- (void)setM22:(float)m22
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    m_matrix3x3f.get()->setM22(m22);
}

- (float)m23
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    return m_matrix3x3f.get()->getM23();
}

- (void)setM23:(float)m23
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    m_matrix3x3f.get()->setM23(m23);
}

- (float)m30
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    return m_matrix3x3f.get()->getM30();
}

- (void)setM30:(float)m30
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    m_matrix3x3f.get()->setM30(m30);
}

- (float)m31
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    return m_matrix3x3f.get()->getM31();
}

- (void)setM31:(float)m31
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    m_matrix3x3f.get()->setM31(m31);
}

- (float)m32
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    return m_matrix3x3f.get()->getM32();
}

- (void)setM32:(float)m32
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    m_matrix3x3f.get()->setM32(m32);
}

- (float)m33
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    return m_matrix3x3f.get()->getM33();
}

- (void)setM33:(float)m33
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    m_matrix3x3f.get()->setM33(m33);
}

- (instancetype)initWithManagedMatrix3x3f:(std::shared_ptr<psm::Matrix3x3f>)matrix3x3f
{
    NSAssert(matrix3x3f.get() != nullptr, @"matrix3x3f is null");
    self = [super init];
    if (!self) {
        return nil;
    }
    m_matrix3x3f = std::move(matrix3x3f);
    return self;
}

- (instancetype)initWithNativeMatrix3x3f:(psm::Matrix3x3f*)matrix3x3f
{
    NSAssert(matrix3x3f != nullptr, @"matrix3x3f is null");
    self = [super init];
    if (!self) {
        return nil;
    }
    try {
        m_matrix3x3f.reset(matrix3x3f);
    } catch (...) {
        return nil;
    }
    return self;
}

- (void*)managedMatrix3x3f
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    return &m_matrix3x3f;
}

- (void*)nativeMatrix3x3f
{
    NSAssert(m_matrix3x3f.get() != nullptr, @"m_matrix3x3f is null");
    return m_matrix3x3f.get();
}

@end
