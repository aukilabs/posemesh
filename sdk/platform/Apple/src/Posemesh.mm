#import <Posemesh/Posemesh.h>

#include <cstring>
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

- (BOOL)sendMessage:(NSData*)message toPeerId:(NSString*)peerId usingProtocol:(NSString*)protocol {
    NSAssert(m_posemesh, @"m_posemesh is null");
    NSAssert(message, @"message is null");
    NSAssert(peerId, @"peerId is null");
    NSAssert(protocol, @"protocol is null");
    return static_cast<BOOL>(m_posemesh->sendMessage([message bytes], [message length], [peerId UTF8String], [protocol UTF8String]));
}

- (BOOL)sendMessage:(NSData*)message toPeerId:(NSString*)peerId usingProtocol:(NSString*)protocol withCallback:(PSMPosemeshSendMessageCallback)callback {
    NSAssert(m_posemesh, @"m_posemesh is null");
    NSAssert(message, @"message is null");
    NSAssert(peerId, @"peerId is null");
    NSAssert(protocol, @"protocol is null");
    return static_cast<BOOL>(m_posemesh->sendMessage([message bytes], [message length], [peerId UTF8String], [protocol UTF8String], callback ? [callback = std::move(callback)](bool status) -> void {
        callback(static_cast<BOOL>(status));
    } : std::function<void(bool status)>{}));
}

- (BOOL)sendString:(NSString*)string withAppendedTerminatingNullCharacter:(BOOL)appendTerminatingNullCharacter toPeerId:(NSString*)peerId usingProtocol:(NSString*)protocol {
    NSAssert(m_posemesh, @"m_posemesh is null");
    NSAssert(string, @"string is null");
    NSAssert(peerId, @"peerId is null");
    NSAssert(protocol, @"protocol is null");
    const char* message = [string UTF8String];
    const std::size_t length = std::strlen(message);
    return static_cast<BOOL>(m_posemesh->sendMessage(message, length + (appendTerminatingNullCharacter ? 1 : 0), [peerId UTF8String], [protocol UTF8String]));
}

- (BOOL)sendString:(NSString*)string withAppendedTerminatingNullCharacter:(BOOL)appendTerminatingNullCharacter toPeerId:(NSString*)peerId usingProtocol:(NSString*)protocol withCallback:(PSMPosemeshSendMessageCallback)callback {
    NSAssert(m_posemesh, @"m_posemesh is null");
    NSAssert(string, @"string is null");
    NSAssert(peerId, @"peerId is null");
    NSAssert(protocol, @"protocol is null");
    const char* message = [string UTF8String];
    const std::size_t length = std::strlen(message);
    return static_cast<BOOL>(m_posemesh->sendMessage(message, length + (appendTerminatingNullCharacter ? 1 : 0), [peerId UTF8String], [protocol UTF8String], callback ? [callback = std::move(callback)](bool status) -> void {
        callback(static_cast<BOOL>(status));
    } : std::function<void(bool status)>{}));
}

- (void*)nativePosemesh {
    NSAssert(m_posemesh, @"m_posemesh is null");
    return m_posemesh;
}

@end
