posemeshModule.QRDetection = null;

__internalPosemeshAPI.builderFunctions.push(function() {
    __internalPosemesh.QRDetection.detectQRFromLuminance = function(imageBytes, width, height, outContents, outCorners) {
        let imageBytesVector = undefined, outContentsVector = undefined, outCornersVector = undefined;
        try {
            imageBytesVector = __internalPosemeshAPI.toVectorUint8(imageBytes);
            outContentsVector = new __internalPosemesh.VectorString();
            outCornersVector = new __internalPosemesh.VectorVector2();
            let result = __internalPosemesh.QRDetection.__detectQRFromLuminance(imageBytesVector, width, height, outContentsVector, outCornersVector);

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

    __internalPosemesh.QRDetection.detectQRFromLuminanceLandmarkObservations = function(imageBytes, width, height) {
        let imageBytesVector = undefined;
        try {
            imageBytesVector = __internalPosemeshAPI.toVectorUint8(imageBytes)
            let result = []
            let observations = __internalPosemesh.QRDetection.__detectQRFromLuminanceLandmarkObservations(imageBytesVector, width, height)

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

    posemeshModule.QRDetection = __internalPosemesh.QRDetection;
});
