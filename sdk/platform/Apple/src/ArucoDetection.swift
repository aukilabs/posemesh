extension ArucoDetection {
    public static func detectAruco(fromLuminanceImageData imageData: Data,
                                width: Int32,
                                height: Int32,
                                markerFormat: ArucoMarkerFormat,
                                outContents: inout [String],
                                outCorners: inout [Vector2]) -> Bool {
        let contents = NSMutableArray();
        let corners = NSMutableArray();
        let result = __detectAruco(fromLuminanceImageData:imageData, ofWidth:width, andHeight:height, for:markerFormat, withOutContents:contents, andOutCorners:corners);
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

    public static func detectAruco(fromLuminanceImageData imageData: Data,
                            width: Int32,
                            height: Int32,
                            markerFormat: ArucoMarkerFormat) -> [LandmarkObservation] {
        return __detectAruco(fromLuminanceImageData:imageData, ofWidth:width, andHeight:height, for:markerFormat) as! [LandmarkObservation];
    }
}
