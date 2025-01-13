#import <Foundation/Foundation.h>

#include "Util.hpp"

namespace psm::util {

std::string getAppSupportDirectoryPath() {
    NSArray<NSURL*>* result = [[NSFileManager defaultManager] URLsForDirectory:NSApplicationSupportDirectory inDomains:NSUserDomainMask];
    if (!result || [result count] < 1) {
        return {};
    }
    NSURL* url = [result firstObject];
    if (!url) {
        return {};
    }
    url = [url URLByAppendingPathComponent:[[NSBundle mainBundle] bundleIdentifier]];
    NSString* path = [url path];
    if (!path) {
        return {};
    }
    if (![[NSFileManager defaultManager] fileExistsAtPath:path]) {
        NSError* error = nil;
        if (![[NSFileManager defaultManager] createDirectoryAtURL:url withIntermediateDirectories:YES attributes:nil error:&error]) {
            NSLog(@"psm::util::getAppSupportDirectoryPath() failed: %@", error.localizedDescription);
            return {};
        }
    }
    return [path UTF8String];
}

}
