/* This code is automatically generated from Vector3f.json interface. Do not modify it manually as it will be overwritten! */

#import <Foundation/Foundation.h>

#import "API.h"

NS_SWIFT_NAME(Vector3f) PSM_API @interface PSMVector3f : NSObject<NSCopying>

- (instancetype)init;
- (instancetype)initWithVector3f:(PSMVector3f*)vector3f;
- (instancetype)copyWithZone:(NSZone*)zone;
- (void)dealloc;

- (BOOL)isEqual:(id)object;
- (NSUInteger)hash;

- (float)x NS_REFINED_FOR_SWIFT;
- (void)setX:(float)x NS_REFINED_FOR_SWIFT;
- (float)y NS_REFINED_FOR_SWIFT;
- (void)setY:(float)y NS_REFINED_FOR_SWIFT;
- (float)z NS_REFINED_FOR_SWIFT;
- (void)setZ:(float)z NS_REFINED_FOR_SWIFT;

#if defined(POSEMESH_BUILD)
- (void*)managedVector3f;
- (void*)nativeVector3f;
#endif

@end
