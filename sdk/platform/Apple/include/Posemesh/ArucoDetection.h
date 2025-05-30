#import <Foundation/Foundation.h>

#import "API.h"
#import "ArucoMarkerFormat.h"
#import "Vector2.h"

NS_SWIFT_NAME(ArucoDetection) PSM_API @interface PSMArucoDetection : NSObject

- (instancetype)init NS_UNAVAILABLE;
- (instancetype)copy NS_UNAVAILABLE;

+ (BOOL)detectArucoFromLuminanceImageData:(NSData*)imageData
                                  ofWidth:(int32_t)width
                                andHeight:(int32_t)height
                          forMarkerFormat:(PSMArucoMarkerFormat)markerFormat
                          withOutContents:(NSMutableArray<NSString*>*)outContents
                            andOutCorners:(NSMutableArray<PSMVector2*>*)outCorners
    NS_REFINED_FOR_SWIFT;

+ (NSArray*)detectArucoFromLuminanceImageData:(NSData*)imageData
                                      ofWidth:(int32_t)width
                                    andHeight:(int32_t)height
                              forMarkerFormat:(PSMArucoMarkerFormat)markerFormat
    NS_REFINED_FOR_SWIFT;

@end
