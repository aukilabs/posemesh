/* This code is automatically generated from Matrix4x4f.json interface. Do not modify it manually as it will be overwritten! */

#import <Foundation/Foundation.h>

#import "API.h"

NS_SWIFT_NAME(Matrix4x4f) PSM_API @interface PSMMatrix4x4f : NSObject<NSCopying>

- (instancetype)init;
- (instancetype)initWithMatrix4x4f:(PSMMatrix4x4f*)matrix4x4f;
- (instancetype)copyWithZone:(NSZone*)zone;
- (void)dealloc;

- (BOOL)isEqual:(id)object;
- (NSUInteger)hash;

- (float)m00 NS_REFINED_FOR_SWIFT;
- (void)setM00:(float)m00 NS_REFINED_FOR_SWIFT;
- (float)m01 NS_REFINED_FOR_SWIFT;
- (void)setM01:(float)m01 NS_REFINED_FOR_SWIFT;
- (float)m02 NS_REFINED_FOR_SWIFT;
- (void)setM02:(float)m02 NS_REFINED_FOR_SWIFT;
- (float)m03 NS_REFINED_FOR_SWIFT;
- (void)setM03:(float)m03 NS_REFINED_FOR_SWIFT;
- (float)m04 NS_REFINED_FOR_SWIFT;
- (void)setM04:(float)m04 NS_REFINED_FOR_SWIFT;
- (float)m10 NS_REFINED_FOR_SWIFT;
- (void)setM10:(float)m10 NS_REFINED_FOR_SWIFT;
- (float)m11 NS_REFINED_FOR_SWIFT;
- (void)setM11:(float)m11 NS_REFINED_FOR_SWIFT;
- (float)m12 NS_REFINED_FOR_SWIFT;
- (void)setM12:(float)m12 NS_REFINED_FOR_SWIFT;
- (float)m13 NS_REFINED_FOR_SWIFT;
- (void)setM13:(float)m13 NS_REFINED_FOR_SWIFT;
- (float)m14 NS_REFINED_FOR_SWIFT;
- (void)setM14:(float)m14 NS_REFINED_FOR_SWIFT;
- (float)m20 NS_REFINED_FOR_SWIFT;
- (void)setM20:(float)m20 NS_REFINED_FOR_SWIFT;
- (float)m21 NS_REFINED_FOR_SWIFT;
- (void)setM21:(float)m21 NS_REFINED_FOR_SWIFT;
- (float)m22 NS_REFINED_FOR_SWIFT;
- (void)setM22:(float)m22 NS_REFINED_FOR_SWIFT;
- (float)m23 NS_REFINED_FOR_SWIFT;
- (void)setM23:(float)m23 NS_REFINED_FOR_SWIFT;
- (float)m24 NS_REFINED_FOR_SWIFT;
- (void)setM24:(float)m24 NS_REFINED_FOR_SWIFT;
- (float)m30 NS_REFINED_FOR_SWIFT;
- (void)setM30:(float)m30 NS_REFINED_FOR_SWIFT;
- (float)m31 NS_REFINED_FOR_SWIFT;
- (void)setM31:(float)m31 NS_REFINED_FOR_SWIFT;
- (float)m32 NS_REFINED_FOR_SWIFT;
- (void)setM32:(float)m32 NS_REFINED_FOR_SWIFT;
- (float)m33 NS_REFINED_FOR_SWIFT;
- (void)setM33:(float)m33 NS_REFINED_FOR_SWIFT;
- (float)m34 NS_REFINED_FOR_SWIFT;
- (void)setM34:(float)m34 NS_REFINED_FOR_SWIFT;
- (float)m40 NS_REFINED_FOR_SWIFT;
- (void)setM40:(float)m40 NS_REFINED_FOR_SWIFT;
- (float)m41 NS_REFINED_FOR_SWIFT;
- (void)setM41:(float)m41 NS_REFINED_FOR_SWIFT;
- (float)m42 NS_REFINED_FOR_SWIFT;
- (void)setM42:(float)m42 NS_REFINED_FOR_SWIFT;
- (float)m43 NS_REFINED_FOR_SWIFT;
- (void)setM43:(float)m43 NS_REFINED_FOR_SWIFT;
- (float)m44 NS_REFINED_FOR_SWIFT;
- (void)setM44:(float)m44 NS_REFINED_FOR_SWIFT;

#if defined(POSEMESH_BUILD)
- (void*)managedMatrix4x4f;
- (void*)nativeMatrix4x4f;
#endif

@end
