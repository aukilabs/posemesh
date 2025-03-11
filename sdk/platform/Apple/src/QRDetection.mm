#import <Posemesh/QRDetection.h>

#include <Posemesh/QRDetection.hpp>

@implementation PSMQRDetection

+ (BOOL)detectQRFromLuminanceFromImageBytes:(const unsigned char*)imageBytes
                                  withWidth:(int)width
                                  andHeight:(int)height
                                forContents:(NSMutableArray<NSString*>*)contents
                                 andCorners:(NSMutableArray<PSMVector2*>*)corners
{
    NSAssert(imageBytes, @"imageBytes is null");
    NSAssert(contents, @"contents is null");
    NSAssert(corners, @"corners is null");

    const uint8_t* imageBytesRawBytes = static_cast<const uint8_t*>(imageBytes);
    const int imageBytesCount = width * height;
    std::vector<uint8_t> bytes(imageBytesRawBytes, imageBytesRawBytes + imageBytesCount);

    std::vector<std::string> outContents;
    std::vector<psm::Vector2> outCorners;
    BOOL result = psm::QRDetection::detectQRFromLuminance(bytes, width, height, outContents, outCorners);

    if (result) {
        [contents addObject:[NSString stringWithCString:outContents[0].c_str() encoding:[NSString defaultCStringEncoding]]];

        for (int i = 0; i < 4; i++) {
            psm::Vector2 o = outCorners[i];
            PSMVector2 *v = [[PSMVector2 alloc] init];
            [v setX:o.getX()];
            [v setY:o.getY()];
            [corners addObject:v];
        }
    }

    return result;
}

@end
