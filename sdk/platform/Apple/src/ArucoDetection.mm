#import <Posemesh/ArucoDetection.h>

#include <Posemesh/ArucoDetection.hpp>

@implementation PSMArucoDetection

+ (BOOL)detectArucoFromLuminanceImageData:(NSData*)imageData
                               ofWidth:(int32_t)width
                             andHeight:(int32_t)height
                       withOutContents:(NSMutableArray<NSString*>*)outContents
                         andOutCorners:(NSMutableArray<PSMVector2*>*)outCorners
{
    NSAssert(imageData, @"imageData is null");
    NSAssert(outContents, @"outContents is null");
    NSAssert(outCorners, @"outCorners is null");

    std::vector<std::string> contents;
    std::vector<psm::Vector2> corners;
    const bool result = psm::ArucoDetection::detectArucoFromLuminance(static_cast<const std::uint8_t*>([imageData bytes]), [imageData length], width, height, contents, corners);
    if (!result) {
        return NO;
    }

    for (const auto& content : contents) {
        [outContents addObject:[NSString stringWithUTF8String:content.c_str()]];
    }

    for (auto& corner : corners) {
        PSMVector2* outCorner = [[PSMVector2 alloc] init];
        *static_cast<psm::Vector2*>([outCorner nativeVector2]) = std::move(corner);
        [outCorners addObject:outCorner];
    }

    return YES;
}

@end
