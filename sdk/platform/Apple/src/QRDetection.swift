extension QRDetection {
    public static func detectQR(fromLuminanceImageData imageData: Data,
                                width: Int32,
                                height: Int32,
                                outContents: inout [String],
                                outCorners: inout [Vector2]) -> Bool {
        let contents = NSMutableArray();
        let corners = NSMutableArray();
        let result = __detectQR(fromLuminanceImageData:imageData, ofWidth:width, andHeight:height, withOutContents:contents, andOutCorners:corners);
        if (result) {
            outContents.removeAll();
            contents.forEach { content in
                outContents.append(content as! String);
            }
            outCorners.removeAll();
            corners.forEach { corner in
                outCorners.append(corner as! Vector2);
            }
        }
        return result;
    }
}
