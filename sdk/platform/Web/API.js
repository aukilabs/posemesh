var __internalPosemeshAPI = {
    builderFunctions: [],
    fromVectorString: function(vectorString) {
        let size = vectorString.size();
        let array = [];
        for (let i = 0; i < size; ++i) {
            array.push(vectorString.get(i));
        }
        return array;
    },
    toVectorString: function(array) {
        let vectorString = new Posemesh.__mainModule.VectorString();
        try {
            let i = 0;
            for (let item of array) {
                if (typeof item !== "string") {
                    throw new Error(`Array item at index ${i} is not a string.`);
                }
                vectorString.push_back(item);
                i++;
            }
            return vectorString;
        } catch (error) {
            vectorString.delete();
            throw error;
        }
    },
    fromVectorUint8: function(vectorUint8) {
        let size = vectorUint8.size();
        let array = [];
        for (let i = 0; i < size; ++i) {
            array.push(vectorUint8.get(i));
        }
        return new Uint8Array(array);
    },
    toVectorUint8: function(array) {
        let vectorUint8 = new Posemesh.__mainModule.VectorUint8();
        try {
            let i = 0;
            for (let item of array) {
                if (typeof item !== "number" || item < 0 || item > 255) {
                    throw new Error(`Array item at index ${i} is not a byte.`);
                }
                vectorUint8.push_back(item);
                i++;
            }
            return vectorUint8;
        } catch (error) {
            vectorUint8.delete();
            throw error;
        }
    }
};
