posemeshModule.QRDetection = null;

__internalPosemeshAPI.builderFunctions.push(function() {
    __internalPosemesh.QRDetection.detectQRFromLuminance = function(imageBytes, width, height, contents, corners) {
        try {
            imageVector = __internalPosemeshAPI.toVectorUint8(imageBytes, false);
            contentsVector = __internalPosemeshAPI.toVectorString(contents, false);
            cornersVector = __internalPosemeshAPI.toVectorVector2(corners, false);
            let result = __internalPosemesh.QRDetection.__detectQRFromLuminance(imageVector, width, height, contentsVector, cornersVector);

            if (result) {
                contents = contentsVector;
                
                for (let i = 0; i < 4; i++) {
                    corners.push(cornersVector.get(i));
                }
            }

            return result;
        } finally {
            if (imageVector) {
                imageVector.delete();
            }
            if (contentsVector) {
                contentsVector.delete();
            }
            if (cornersVector) {
                cornersVector.delete();
            }
        }
    };

    posemeshModule.QRDetection = __internalPosemesh.QRDetection;
});
