/* This code is automatically generated from Landmark.json interface. Do not modify it manually as it will be overwritten! */

#import <Foundation/Foundation.h>

#import "API.h"
#import "Vector3.h"

NS_SWIFT_NAME(Landmark) PSM_API @interface PSMLandmark : NSObject<NSCopying>

- (instancetype _Nonnull)init;
- (instancetype)initWithLandmark:(PSMLandmark*)landmark;
- (instancetype)copyWithZone:(NSZone*)zone;
- (void)dealloc;

- (BOOL)isEqual:(id)object;
- (NSUInteger)hash;

- (NSString*)type NS_REFINED_FOR_SWIFT;
- (void)setType:(NSString*)type NS_REFINED_FOR_SWIFT;
- (NSString*)id NS_REFINED_FOR_SWIFT;
- (void)setId:(NSString*)id NS_REFINED_FOR_SWIFT;
- (PSMVector3*)position NS_REFINED_FOR_SWIFT;
- (void)setPosition:(PSMVector3*)position NS_REFINED_FOR_SWIFT;

#if defined(POSEMESH_BUILD)
- (void*)managedLandmark;
- (void*)nativeLandmark;
#endif

@end
