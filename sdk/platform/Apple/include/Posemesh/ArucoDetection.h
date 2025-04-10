#import <Foundation/Foundation.h>

#import "API.h"
#import "Vector2.h"

NS_SWIFT_NAME(ArucoDetection) PSM_API @interface PSMArucoDetection : NSObject

- (instancetype)init NS_UNAVAILABLE;
- (instancetype)copy NS_UNAVAILABLE;

+ (BOOL)detectArucoFromLuminanceImageData:(NSData*)imageData
                                  ofWidth:(int32_t)width
                                andHeight:(int32_t)height
                          withOutContents:(NSMutableArray<NSString*>*)outContents
                            andOutCorners:(NSMutableArray<PSMVector2*>*)outCorners
    NS_REFINED_FOR_SWIFT;

@end
