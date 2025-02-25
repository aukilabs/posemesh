/* This code is automatically generated from Matrix4x4f.json interface. Do not modify it manually as it will be overwritten! */

#import <Posemesh/Matrix4x4f.h>

#include <Posemesh/Matrix4x4f.hpp>
#include <memory>
#include <new>
#include <utility>

@implementation PSMMatrix4x4f {
    std::shared_ptr<psm::Matrix4x4f> m_matrix4x4f;
}

- (instancetype)init
{
    auto* matrix4x4f = new (std::nothrow) psm::Matrix4x4f;
    if (!matrix4x4f) {
        return nil;
    }
    self = [self initWithNativeMatrix4x4f:matrix4x4f];
    if (!self) {
        delete matrix4x4f;
        return nil;
    }
    return self;
}

- (instancetype)initWithMatrix4x4f:(PSMMatrix4x4f*)matrix4x4f
{
    NSAssert(matrix4x4f != nil, @"matrix4x4f is null");
    NSAssert(matrix4x4f->m_matrix4x4f.get() != nullptr, @"matrix4x4f->m_matrix4x4f is null");
    auto* copy = new (std::nothrow) psm::Matrix4x4f(*(matrix4x4f->m_matrix4x4f.get()));
    if (!copy) {
        return nil;
    }
    self = [self initWithNativeMatrix4x4f:copy];
    if (!self) {
        delete copy;
        return nil;
    }
    return self;
}

- (instancetype)copyWithZone:(NSZone*)zone
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    auto* matrix4x4f = new (std::nothrow) psm::Matrix4x4f(*(m_matrix4x4f.get()));
    if (!matrix4x4f) {
        return nil;
    }
    PSMMatrix4x4f* copy = [[[self class] allocWithZone:zone] initWithNativeMatrix4x4f:matrix4x4f];
    if (!copy) {
        delete matrix4x4f;
        return nil;
    }
    return copy;
}

- (void)dealloc
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
}

- (BOOL)isEqual:(id)object
{
    if (self == object) {
        return YES;
    }
    if (![object isKindOfClass:[PSMMatrix4x4f class]]) {
        return NO;
    }
    PSMMatrix4x4f* matrix4x4f = (PSMMatrix4x4f*)object;
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    NSAssert(matrix4x4f->m_matrix4x4f.get() != nullptr, @"matrix4x4f->m_matrix4x4f is null");
    return m_matrix4x4f.get()->operator==(*(matrix4x4f->m_matrix4x4f.get()));
}

- (NSUInteger)hash
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return std::hash<psm::Matrix4x4f> {}(*(m_matrix4x4f.get()));
}

- (float)m00
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM00();
}

- (void)setM00:(float)m00
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM00(m00);
}

- (float)m01
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM01();
}

- (void)setM01:(float)m01
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM01(m01);
}

- (float)m02
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM02();
}

- (void)setM02:(float)m02
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM02(m02);
}

- (float)m03
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM03();
}

- (void)setM03:(float)m03
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM03(m03);
}

- (float)m04
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM04();
}

- (void)setM04:(float)m04
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM04(m04);
}

- (float)m10
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM10();
}

- (void)setM10:(float)m10
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM10(m10);
}

- (float)m11
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM11();
}

- (void)setM11:(float)m11
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM11(m11);
}

- (float)m12
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM12();
}

- (void)setM12:(float)m12
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM12(m12);
}

- (float)m13
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM13();
}

- (void)setM13:(float)m13
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM13(m13);
}

- (float)m14
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM14();
}

- (void)setM14:(float)m14
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM14(m14);
}

- (float)m20
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM20();
}

- (void)setM20:(float)m20
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM20(m20);
}

- (float)m21
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM21();
}

- (void)setM21:(float)m21
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM21(m21);
}

- (float)m22
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM22();
}

- (void)setM22:(float)m22
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM22(m22);
}

- (float)m23
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM23();
}

- (void)setM23:(float)m23
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM23(m23);
}

- (float)m24
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM24();
}

- (void)setM24:(float)m24
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM24(m24);
}

- (float)m30
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM30();
}

- (void)setM30:(float)m30
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM30(m30);
}

- (float)m31
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM31();
}

- (void)setM31:(float)m31
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM31(m31);
}

- (float)m32
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM32();
}

- (void)setM32:(float)m32
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM32(m32);
}

- (float)m33
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM33();
}

- (void)setM33:(float)m33
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM33(m33);
}

- (float)m34
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM34();
}

- (void)setM34:(float)m34
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM34(m34);
}

- (float)m40
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM40();
}

- (void)setM40:(float)m40
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM40(m40);
}

- (float)m41
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM41();
}

- (void)setM41:(float)m41
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM41(m41);
}

- (float)m42
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM42();
}

- (void)setM42:(float)m42
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM42(m42);
}

- (float)m43
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM43();
}

- (void)setM43:(float)m43
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM43(m43);
}

- (float)m44
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get()->getM44();
}

- (void)setM44:(float)m44
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    m_matrix4x4f.get()->setM44(m44);
}

- (instancetype)initWithManagedMatrix4x4f:(std::shared_ptr<psm::Matrix4x4f>)matrix4x4f
{
    NSAssert(matrix4x4f.get() != nullptr, @"matrix4x4f is null");
    self = [super init];
    if (!self) {
        return nil;
    }
    m_matrix4x4f = std::move(matrix4x4f);
    return self;
}

- (instancetype)initWithNativeMatrix4x4f:(psm::Matrix4x4f*)matrix4x4f
{
    NSAssert(matrix4x4f != nullptr, @"matrix4x4f is null");
    self = [super init];
    if (!self) {
        return nil;
    }
    try {
        m_matrix4x4f.reset(matrix4x4f);
    } catch (...) {
        return nil;
    }
    return self;
}

- (void*)managedMatrix4x4f
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return &m_matrix4x4f;
}

- (void*)nativeMatrix4x4f
{
    NSAssert(m_matrix4x4f.get() != nullptr, @"m_matrix4x4f is null");
    return m_matrix4x4f.get();
}

@end
