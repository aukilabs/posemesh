#import <Foundation/Foundation.h>

#import "API.h"

NS_SWIFT_NAME(Config) PSM_API @interface PSMConfig : NSObject<NSCopying>

- (instancetype)init;
- (instancetype)copyWithZone:(NSZone*)zone;
- (void)dealloc;

- (BOOL)serveAsBootstrap NS_REFINED_FOR_SWIFT;
- (void)setServeAsBootstrap:(BOOL)serveAsBootstrap NS_REFINED_FOR_SWIFT;

- (BOOL)serveAsRelay NS_REFINED_FOR_SWIFT;
- (void)setServeAsRelay:(BOOL)serveAsRelay NS_REFINED_FOR_SWIFT;

@end
