/* This code is automatically generated from Matrix3x3f.json interface. Do not modify it manually as it will be overwritten! */

#import <Foundation/Foundation.h>

#import "API.h"

NS_SWIFT_NAME(Matrix3x3f) PSM_API @interface PSMMatrix3x3f : NSObject<NSCopying>

- (instancetype)init;
- (instancetype)initWithMatrix3x3f:(PSMMatrix3x3f*)matrix3x3f;
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
- (float)m10 NS_REFINED_FOR_SWIFT;
- (void)setM10:(float)m10 NS_REFINED_FOR_SWIFT;
- (float)m11 NS_REFINED_FOR_SWIFT;
- (void)setM11:(float)m11 NS_REFINED_FOR_SWIFT;
- (float)m12 NS_REFINED_FOR_SWIFT;
- (void)setM12:(float)m12 NS_REFINED_FOR_SWIFT;
- (float)m13 NS_REFINED_FOR_SWIFT;
- (void)setM13:(float)m13 NS_REFINED_FOR_SWIFT;
- (float)m20 NS_REFINED_FOR_SWIFT;
- (void)setM20:(float)m20 NS_REFINED_FOR_SWIFT;
- (float)m21 NS_REFINED_FOR_SWIFT;
- (void)setM21:(float)m21 NS_REFINED_FOR_SWIFT;
- (float)m22 NS_REFINED_FOR_SWIFT;
- (void)setM22:(float)m22 NS_REFINED_FOR_SWIFT;
- (float)m23 NS_REFINED_FOR_SWIFT;
- (void)setM23:(float)m23 NS_REFINED_FOR_SWIFT;
- (float)m30 NS_REFINED_FOR_SWIFT;
- (void)setM30:(float)m30 NS_REFINED_FOR_SWIFT;
- (float)m31 NS_REFINED_FOR_SWIFT;
- (void)setM31:(float)m31 NS_REFINED_FOR_SWIFT;
- (float)m32 NS_REFINED_FOR_SWIFT;
- (void)setM32:(float)m32 NS_REFINED_FOR_SWIFT;
- (float)m33 NS_REFINED_FOR_SWIFT;
- (void)setM33:(float)m33 NS_REFINED_FOR_SWIFT;

#if defined(POSEMESH_BUILD)
- (void*)managedMatrix3x3f;
- (void*)nativeMatrix3x3f;
#endif

@end
