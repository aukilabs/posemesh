/* This code is automatically generated from Landmark.json interface. Do not modify it manually as it will be overwritten! */

#import <Posemesh/Landmark.h>

#include <Posemesh/Landmark.hpp>
#include <memory>
#include <new>
#include <utility>

@implementation PSMLandmark {
    std::shared_ptr<psm::Landmark> m_landmark;
}

- (instancetype)init
{
    auto* landmark = new (std::nothrow) psm::Landmark;
    if (!landmark) {
        return nil;
    }
    self = [self initWithNativeLandmark:landmark];
    if (!self) {
        delete landmark;
        return nil;
    }
    return self;
}

- (instancetype)initWithLandmark:(PSMLandmark*)landmark
{
    NSAssert(landmark != nil, @"landmark is null");
    NSAssert(landmark->m_landmark.get() != nullptr, @"landmark->m_landmark is null");
    auto* copy = new (std::nothrow) psm::Landmark(*(landmark->m_landmark.get()));
    if (!copy) {
        return nil;
    }
    self = [self initWithNativeLandmark:copy];
    if (!self) {
        delete copy;
        return nil;
    }
    return self;
}

- (instancetype)copyWithZone:(NSZone*)zone
{
    NSAssert(m_landmark.get() != nullptr, @"m_landmark is null");
    auto* landmark = new (std::nothrow) psm::Landmark(*(m_landmark.get()));
    if (!landmark) {
        return nil;
    }
    PSMLandmark* copy = [[[self class] allocWithZone:zone] initWithNativeLandmark:landmark];
    if (!copy) {
        delete landmark;
        return nil;
    }
    return copy;
}

- (void)dealloc
{
    NSAssert(m_landmark.get() != nullptr, @"m_landmark is null");
}

- (BOOL)isEqual:(id)object
{
    if (self == object) {
        return YES;
    }
    if (![object isKindOfClass:[PSMLandmark class]]) {
        return NO;
    }
    PSMLandmark* landmark = (PSMLandmark*)object;
    NSAssert(m_landmark.get() != nullptr, @"m_landmark is null");
    NSAssert(landmark->m_landmark.get() != nullptr, @"landmark->m_landmark is null");
    return m_landmark.get()->operator==(*(landmark->m_landmark.get()));
}

- (NSUInteger)hash
{
    NSAssert(m_landmark.get() != nullptr, @"m_landmark is null");
    return std::hash<psm::Landmark> {}(*(m_landmark.get()));
}

- (NSString*)type
{
    NSAssert(m_landmark.get() != nullptr, @"m_landmark is null");
    return [NSString stringWithUTF8String:m_landmark.get()->getType().c_str()];
}

- (void)setType:(NSString*)type
{
    NSAssert(m_landmark.get() != nullptr, @"m_landmark is null");
    m_landmark.get()->setType([type UTF8String]);
}

- (NSString*)id
{
    NSAssert(m_landmark.get() != nullptr, @"m_landmark is null");
    return [NSString stringWithUTF8String:m_landmark.get()->getId().c_str()];
}

- (void)setId:(NSString*)id
{
    NSAssert(m_landmark.get() != nullptr, @"m_landmark is null");
    m_landmark.get()->setId([id UTF8String]);
}

- (PSMVector3*)position
{
    NSAssert(m_landmark.get() != nullptr, @"m_landmark is null");
    psm::Vector3 p = m_landmark.get()->getPosition();
    PSMVector3* position = [[PSMVector3 alloc] init];
    [position setX:p.getX()];
    [position setY:p.getY()];
    [position setZ:p.getZ()];
    return position;
}

- (void)setPosition:(PSMVector3*)position
{
    NSAssert(m_landmark.get() != nullptr, @"m_landmark is null");
    psm::Vector3 newPosition;
    newPosition.setX([position x]);
    newPosition.setY([position y]);
    newPosition.setZ([position z]);
    m_landmark.get()->setPosition(newPosition);
}

- (instancetype)initWithManagedLandmark:(std::shared_ptr<psm::Landmark>)landmark
{
    NSAssert(landmark.get() != nullptr, @"landmark is null");
    self = [super init];
    if (!self) {
        return nil;
    }
    m_landmark = std::move(landmark);
    return self;
}

- (instancetype)initWithNativeLandmark:(psm::Landmark*)landmark
{
    NSAssert(landmark != nullptr, @"landmark is null");
    self = [super init];
    if (!self) {
        return nil;
    }
    try {
        m_landmark.reset(landmark);
    } catch (...) {
        return nil;
    }
    return self;
}

- (void*)managedLandmark
{
    NSAssert(m_landmark.get() != nullptr, @"m_landmark is null");
    return &m_landmark;
}

- (void*)nativeLandmark
{
    NSAssert(m_landmark.get() != nullptr, @"m_landmark is null");
    return m_landmark.get();
}

@end
