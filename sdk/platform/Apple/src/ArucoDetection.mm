#import <Posemesh/ArucoDetection.h>
#import <Posemesh/ArucoDetection.hpp>
#import <Posemesh/LandmarkObservation.h>

@implementation PSMArucoDetection

+ (BOOL)detectArucoFromLuminanceImageData:(NSData*)imageData
                                  ofWidth:(int32_t)width
                                andHeight:(int32_t)height
                          forMarkerFormat:(PSMArucoMarkerFormat)markerFormat
                          withOutContents:(NSMutableArray<NSString*>*)outContents
                            andOutCorners:(NSMutableArray<PSMVector2*>*)outCorners
{
    NSAssert(imageData, @"imageData is null");
    NSAssert(outContents, @"outContents is null");
    NSAssert(outCorners, @"outCorners is null");

    std::vector<std::string> contents;
    std::vector<psm::Vector2> corners;
    const bool result = psm::ArucoDetection::detectArucoFromLuminance(static_cast<const std::uint8_t*>([imageData bytes]), [imageData length], width, height, (psm::ArucoMarkerFormat)markerFormat, contents, corners);
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

+ (NSArray*)detectArucoFromLuminanceImageData:(NSData*)imageData
                                      ofWidth:(int32_t)width
                                    andHeight:(int32_t)height
                              forMarkerFormat:(PSMArucoMarkerFormat)markerFormat
{
    NSAssert(imageData, @"imageData is null");
    NSAssert([imageData length] == width * height, @"imageData size does not correspond to width & height");
    const uint8_t* bytes = static_cast<const std::uint8_t*>([imageData bytes]);
    std::vector<uint8_t> data(bytes, bytes + width * height);
    std::vector<psm::LandmarkObservation> r = psm::ArucoDetection::detectArucoFromLuminance(data, width, height, (psm::ArucoMarkerFormat)markerFormat);

    NSMutableArray* result = [[NSMutableArray alloc] init];

    for (auto& corner : r) {
        PSMLandmarkObservation* o = [[PSMLandmarkObservation alloc] init];
        *static_cast<psm::LandmarkObservation*>([o nativeLandmarkObservation]) = std::move(corner);
        [result addObject:o];
    }

    return result;
}

@end
