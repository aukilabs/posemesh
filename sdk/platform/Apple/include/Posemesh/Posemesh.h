#import <Foundation/Foundation.h>

#import "API.h"
#import "Config.h"

typedef void (^PSMPosemeshSendMessageCallback)(BOOL status) NS_SWIFT_NAME(Posemesh.SendMessageCallback);

NS_SWIFT_NAME(Posemesh) PSM_API @interface PSMPosemesh : NSObject

- (instancetype)init;
- (instancetype)initWithConfig:(PSMConfig*)config;
- (instancetype)copy NS_UNAVAILABLE;
- (void)dealloc;

- (BOOL)isEqual:(id)object;
- (NSUInteger)hash;

- (BOOL)sendMessage:(NSData*)message toPeerId:(NSString*)peerId usingProtocol:(NSString*)protocol;
- (BOOL)sendMessage:(NSData*)message toPeerId:(NSString*)peerId usingProtocol:(NSString*)protocol withCallback:(PSMPosemeshSendMessageCallback)callback;

- (BOOL)sendString:(NSString*)string withAppendedTerminatingNullCharacter:(BOOL)appendTerminatingNullCharacter toPeerId:(NSString*)peerId usingProtocol:(NSString*)protocol;
- (BOOL)sendString:(NSString*)string withAppendedTerminatingNullCharacter:(BOOL)appendTerminatingNullCharacter toPeerId:(NSString*)peerId usingProtocol:(NSString*)protocol withCallback:(PSMPosemeshSendMessageCallback)callback;

- (void)pnpSolveForObjectPoints:(float *)objectPoints imagePoints:(float *)imagePoints cameraMatrix:(float *)cameraMatrix outR:(float *)outR outT:(float *)outT;

#if defined(POSEMESH_BUILD)
- (void*)nativePosemesh;
#endif

+ (NSString*)getVersion;
+ (NSString*)getCommitId;

@end
