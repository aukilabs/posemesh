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
    }
};
