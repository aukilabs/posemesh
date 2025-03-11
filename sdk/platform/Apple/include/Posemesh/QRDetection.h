#import <Foundation/Foundation.h>

#import "API.h"
#import "Matrix3x3.h"
#import "Vector2.h"
#import "Vector3.h"

NS_SWIFT_NAME(QRDetection) PSM_API @interface PSMQRDetection : NSObject

- (instancetype)init NS_UNAVAILABLE;
- (instancetype)copy NS_UNAVAILABLE;

+ (BOOL)detectQRFromLuminanceFromImageBytes:(const unsigned char*)imageBytes
                                  withWidth:(int)width
                                  andHeight:(int)height
                                forContents:(NSMutableArray<NSString*>*)contents
                                 andCorners:(NSMutableArray<PSMVector2*>*)corners;

@end
