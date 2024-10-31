#import <Foundation/Foundation.h>

#import "API.h"
#import "Config.h"

NS_SWIFT_NAME(Posemesh) PSM_API @interface PSMPosemesh : NSObject

- (instancetype)init;
- (instancetype)initWithConfig:(PSMConfig*)config;
- (instancetype)copy NS_UNAVAILABLE;
- (void)dealloc;

- (BOOL)isEqual:(id)object;
- (NSUInteger)hash;

#if defined(POSEMESH_BUILD)
- (void*)nativePosemesh;
#endif

@end
