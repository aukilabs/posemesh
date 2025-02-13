/* This code is automatically generated from Vector2f.json interface. Do not modify it manually as it will be overwritten! */

#import <Foundation/Foundation.h>

#import "API.h"

NS_SWIFT_NAME(Vector2f) PSM_API @interface PSMVector2f : NSObject<NSCopying>

- (instancetype)init;
- (instancetype)initWithVector2f:(PSMVector2f*)vector2f;
- (instancetype)copyWithZone:(NSZone*)zone;
- (void)dealloc;

- (BOOL)isEqual:(id)object;
- (NSUInteger)hash;

- (float)x NS_REFINED_FOR_SWIFT;
- (void)setX:(float)x NS_REFINED_FOR_SWIFT;
- (float)y NS_REFINED_FOR_SWIFT;
- (void)setY:(float)y NS_REFINED_FOR_SWIFT;
- (float)length NS_REFINED_FOR_SWIFT;

#if defined(POSEMESH_BUILD)
- (void*)managedVector2f;
- (void*)nativeVector2f;
#endif

@end
