#import <Foundation/Foundation.h>

#import "API.h"

NS_SWIFT_NAME(Config) PSM_API @interface PSMConfig : NSObject<NSCopying>

- (instancetype)init;
- (instancetype)initWithConfig:(PSMConfig*)config;
- (instancetype)copyWithZone:(NSZone*)zone;
- (void)dealloc;

- (BOOL)isEqual:(id)object;
- (NSUInteger)hash;

- (NSArray<NSString*>*)bootstraps NS_REFINED_FOR_SWIFT;
- (BOOL)setBootstraps:(NSArray<NSString*>*)bootstraps;

- (NSArray<NSString*>*)relays NS_REFINED_FOR_SWIFT;
- (BOOL)setRelays:(NSArray<NSString*>*)relays;

- (NSData*)privateKey NS_REFINED_FOR_SWIFT;
- (void)setPrivateKey:(NSData*)privateKey NS_REFINED_FOR_SWIFT;

- (NSString*)privateKeyPath NS_REFINED_FOR_SWIFT;
- (void)setPrivateKeyPath:(NSString*)privateKeyPath NS_REFINED_FOR_SWIFT;

- (BOOL)enableMDNS NS_REFINED_FOR_SWIFT;
- (void)setEnableMDNS:(BOOL)enableMDNS NS_REFINED_FOR_SWIFT;

- (NSString*)name NS_REFINED_FOR_SWIFT;
- (void)setName:(NSString*)name NS_REFINED_FOR_SWIFT;

#if defined(POSEMESH_BUILD)
- (void*)nativeConfig;
#endif

+ (PSMConfig*)default NS_REFINED_FOR_SWIFT;

@end
