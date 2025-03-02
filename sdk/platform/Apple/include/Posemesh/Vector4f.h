/* This code is automatically generated from Vector4f.json interface. Do not modify it manually as it will be overwritten! */

#import <Foundation/Foundation.h>

#import "API.h"

NS_SWIFT_NAME(Vector4f) PSM_API @interface PSMVector4f : NSObject<NSCopying>

- (instancetype)init;
- (instancetype)initWithVector4f:(PSMVector4f*)vector4f;
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
- (float)w NS_REFINED_FOR_SWIFT;
- (void)setW:(float)w NS_REFINED_FOR_SWIFT;

#if defined(POSEMESH_BUILD)
- (void*)managedVector4f;
- (void*)nativeVector4f;
#endif

@end

#if defined(__swift__)
typedef PSMVector4f* __PSMQuaternion NS_SWIFT_NAME(Quaternion);
#else
@compatibility_alias PSMQuaternion PSMVector4f;
#endif
