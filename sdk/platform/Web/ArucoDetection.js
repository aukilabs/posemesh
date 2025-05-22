posemeshModule.ArucoDetection = null;

__internalPosemeshAPI.builderFunctions.push(function() {
    __internalPosemesh.ArucoDetection.detectArucoFromLuminance = function(imageBytes, width, height, markerFormat, outContents, outCorners) {
        let imageBytesVector = undefined, outContentsVector = undefined, outCornersVector = undefined;
        try {
            imageBytesVector = __internalPosemeshAPI.toVectorUint8(imageBytes);
            outContentsVector = new __internalPosemesh.VectorString();
            outCornersVector = new __internalPosemesh.VectorVector2();
            let result = __internalPosemesh.ArucoDetection.__detectArucoFromLuminance(imageBytesVector, width, height, markerFormat, outContentsVector, outCornersVector);

            if (result) {
                outContents.length = 0;
                let outContentsVectorSize = outContentsVector.size();
                for (let i = 0; i < outContentsVectorSize; i++) {
                    outContents.push(outContentsVector.get(i));
                }

                for (let corner of outCorners) {
                    corner.delete();
                }
                outCorners.length = 0;
                let outCornersVectorSize = outCornersVector.size();
                for (let i = 0; i < outCornersVectorSize; i++) {
                    outCorners.push(outCornersVector.get(i));
                }
            }

            return result;
        } finally {
            if (outCornersVector) {
                outCornersVector.delete();
            }
            if (outContentsVector) {
                outContentsVector.delete();
            }
            if (imageBytesVector) {
                imageBytesVector.delete();
            }
        }
    };

    __internalPosemesh.ArucoDetection.detectArucoFromLuminanceLandmarkObservations = function(imageBytes, width, height, markerFormat) {
        let imageBytesVector = undefined;
        try {
            imageBytesVector = __internalPosemeshAPI.toVectorUint8(imageBytes)
            let result = []
            let observations = __internalPosemesh.ArucoDetection.__detectArucoFromLuminanceLandmarkObservations(imageBytesVector, width, height, markerFormat)

            for (let i = 0; i < observations.size(); i++) {
                result.push(observations.get(i))
            }

            return result;
        } finally {
            if (imageBytesVector) {
                imageBytesVector.delete();
            }
        }
    };

    posemeshModule.ArucoDetection = __internalPosemesh.ArucoDetection;
});
