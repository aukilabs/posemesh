extension QRDetection {
    public static func detectQRFromLuminance(fromImageBytes imageBytes: [UInt8],
                                             width: Int32,
                                             height: Int32,
                                             contents: inout NSMutableArray,
                                             corners: inout NSMutableArray) -> Bool {
        detectQRFromLuminance(fromImageBytes:imageBytes, withWidth:width, andHeight:height, forContents:contents, andCorners:corners);
    }
}
