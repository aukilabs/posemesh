var __internalPosemeshAPI = {
    builderFunctions: [
        function(context) {
            __internalPosemesh.VectorBoolean = __internalPosemesh.VectorUint8;
        }
    ],

    fromVectorInt8: function(vectorInt8) {
        let size = vectorInt8.size();
        let array = new Int8Array(size);
        for (let i = 0; i < size; ++i) {
            array[i] = vectorInt8.get(i);
        }
        return array;
    },
    toVectorInt8: function(array) {
        let vectorInt8 = new __internalPosemesh.VectorInt8();
        vectorInt8.resize(array.length);
        try {
            if (array instanceof Int8Array) {
                let i = 0;
                for (let item of array) {
                    vectorInt8.set(i, item);
                    i++;
                }
            } else {
                let i = 0;
                for (let item of array) {
                    if (typeof item === 'bigint') {
                        if (item >= -128n && item <= 127n) {
                            vectorInt8.set(i, Number(item));
                            i++;
                            continue;
                        }
                    } else if (typeof item === 'number') {
                        if (item >= -128 && item <= 127) {
                            vectorInt8.set(i, item);
                            i++;
                            continue;
                        }
                    }
                    throw new Error(`Array item at index ${i} is not a signed 8-bit integer.`);
                }
            }
            return vectorInt8;
        } catch (error) {
            vectorInt8.delete();
            throw error;
        }
    },

    fromVectorInt16: function(vectorInt16) {
        let size = vectorInt16.size();
        let array = new Int16Array(size);
        for (let i = 0; i < size; ++i) {
            array[i] = vectorInt16.get(i);
        }
        return array;
    },
    toVectorInt16: function(array) {
        let vectorInt16 = new __internalPosemesh.VectorInt16();
        vectorInt16.resize(array.length);
        try {
            if (array instanceof Int16Array || array instanceof Int8Array || array instanceof Uint8Array) {
                let i = 0;
                for (let item of array) {
                    vectorInt16.set(i, item);
                    i++;
                }
            } else {
                let i = 0;
                for (let item of array) {
                    if (typeof item === 'bigint') {
                        if (item >= -32768n && item <= 32767n) {
                            vectorInt16.set(i, Number(item));
                            i++;
                            continue;
                        }
                    } else if (typeof item === 'number') {
                        if (item >= -32768 && item <= 32767) {
                            vectorInt16.set(i, item);
                            i++;
                            continue;
                        }
                    }
                    throw new Error(`Array item at index ${i} is not a signed 16-bit integer.`);
                }
            }
            return vectorInt16;
        } catch (error) {
            vectorInt16.delete();
            throw error;
        }
    },

    fromVectorInt32: function(vectorInt32) {
        let size = vectorInt32.size();
        let array = new Int32Array(size);
        for (let i = 0; i < size; ++i) {
            array[i] = vectorInt32.get(i);
        }
        return array;
    },
    toVectorInt32: function(array) {
        let vectorInt32 = new __internalPosemesh.VectorInt32();
        vectorInt32.resize(array.length);
        try {
            if (array instanceof Int32Array || array instanceof Int16Array || array instanceof Int8Array || array instanceof Uint16Array || array instanceof Uint8Array) {
                let i = 0;
                for (let item of array) {
                    vectorInt32.set(i, item);
                    i++;
                }
            } else {
                let i = 0;
                for (let item of array) {
                    if (typeof item === 'bigint') {
                        if (item >= -2147483648n && item <= 2147483647n) {
                            vectorInt32.set(i, Number(item));
                            i++;
                            continue;
                        }
                    } else if (typeof item === 'number') {
                        if (item >= -2147483648 && item <= 2147483647) {
                            vectorInt32.set(i, item);
                            i++;
                            continue;
                        }
                    }
                    throw new Error(`Array item at index ${i} is not a signed 32-bit integer.`);
                }
            }
            return vectorInt32;
        } catch (error) {
            vectorInt32.delete();
            throw error;
        }
    },

    fromVectorInt64: function(vectorInt64) {
        let size = vectorInt64.size();
        let array = new BigInt64Array(size);
        for (let i = 0; i < size; ++i) {
            array[i] = vectorInt64.get(i);
        }
        return array;
    },
    toVectorInt64: function(array) {
        let vectorInt64 = new __internalPosemesh.VectorInt64();
        vectorInt64.resize(array.length);
        try {
            if (array instanceof BigInt64Array) {
                let i = 0;
                for (let item of array) {
                    vectorInt64.set(i, item);
                    i++;
                }
            } else if (array instanceof Int32Array || array instanceof Int16Array || array instanceof Int8Array || array instanceof Uint32Array || array instanceof Uint16Array || array instanceof Uint8Array) {
                let i = 0;
                for (let item of array) {
                    vectorInt64.set(i, BigInt(item));
                    i++;
                }
            } else {
                let i = 0;
                for (let item of array) {
                    if (typeof item === 'bigint') {
                        if (item >= -9223372036854775808n && item <= 9223372036854775807n) {
                            vectorInt64.set(i, item);
                            i++;
                            continue;
                        }
                    } else if (typeof item === 'number') {
                        let bigIntItem = BigInt(item);
                        if (bigIntItem >= -9223372036854775808n && bigIntItem <= 9223372036854775807n) {
                            vectorInt64.set(i, bigIntItem);
                            i++;
                            continue;
                        }
                    }
                    throw new Error(`Array item at index ${i} is not a signed 64-bit integer.`);
                }
            }
            return vectorInt64;
        } catch (error) {
            vectorInt64.delete();
            throw error;
        }
    },

    fromVectorUint8: function(vectorUint8) {
        let size = vectorUint8.size();
        let array = new Uint8Array(size);
        for (let i = 0; i < size; ++i) {
            array[i] = vectorUint8.get(i);
        }
        return array;
    },
    toVectorUint8: function(array) {
        let vectorUint8 = new __internalPosemesh.VectorUint8();
        vectorUint8.resize(array.length);
        try {
            if (array instanceof Uint8Array) {
                let i = 0;
                for (let item of array) {
                    vectorUint8.set(i, item);
                    i++;
                }
            } else {
                let i = 0;
                for (let item of array) {
                    if (typeof item === 'bigint') {
                        if (item >= 0n && item <= 255n) {
                            vectorUint8.set(i, Number(item));
                            i++;
                            continue;
                        }
                    } else if (typeof item === 'number') {
                        if (item >= 0 && item <= 255) {
                            vectorUint8.set(i, item);
                            i++;
                            continue;
                        }
                    }
                    throw new Error(`Array item at index ${i} is not an unsigned 8-bit integer.`);
                }
            }
            return vectorUint8;
        } catch (error) {
            vectorUint8.delete();
            throw error;
        }
    },

    fromVectorUint16: function(vectorUint16) {
        let size = vectorUint16.size();
        let array = new Uint16Array(size);
        for (let i = 0; i < size; ++i) {
            array[i] = vectorUint16.get(i);
        }
        return array;
    },
    toVectorUint16: function(array) {
        let vectorUint16 = new __internalPosemesh.VectorUint16();
        vectorUint16.resize(array.length);
        try {
            if (array instanceof Uint16Array || array instanceof Uint8Array) {
                let i = 0;
                for (let item of array) {
                    vectorUint16.set(i, item);
                    i++;
                }
            } else {
                let i = 0;
                for (let item of array) {
                    if (typeof item === 'bigint') {
                        if (item >= 0n && item <= 65535n) {
                            vectorUint16.set(i, Number(item));
                            i++;
                            continue;
                        }
                    } else if (typeof item === 'number') {
                        if (item >= 0 && item <= 65535) {
                            vectorUint16.set(i, item);
                            i++;
                            continue;
                        }
                    }
                    throw new Error(`Array item at index ${i} is not an unsigned 16-bit integer.`);
                }
            }
            return vectorUint16;
        } catch (error) {
            vectorUint16.delete();
            throw error;
        }
    },

    fromVectorUint32: function(vectorUint32) {
        let size = vectorUint32.size();
        let array = new Uint32Array(size);
        for (let i = 0; i < size; ++i) {
            array[i] = vectorUint32.get(i);
        }
        return array;
    },
    toVectorUint32: function(array) {
        let vectorUint32 = new __internalPosemesh.VectorUint32();
        vectorUint32.resize(array.length);
        try {
            if (array instanceof Uint32Array || array instanceof Uint16Array || array instanceof Uint8Array) {
                let i = 0;
                for (let item of array) {
                    vectorUint32.set(i, item);
                    i++;
                }
            } else {
                let i = 0;
                for (let item of array) {
                    if (typeof item === 'bigint') {
                        if (item >= 0n && item <= 4294967295n) {
                            vectorUint32.set(i, Number(item));
                            i++;
                            continue;
                        }
                    } else if (typeof item === 'number') {
                        if (item >= 0 && item <= 4294967295) {
                            vectorUint32.set(i, item);
                            i++;
                            continue;
                        }
                    }
                    throw new Error(`Array item at index ${i} is not an unsigned 32-bit integer.`);
                }
            }
            return vectorUint32;
        } catch (error) {
            vectorUint32.delete();
            throw error;
        }
    },

    fromVectorUint64: function(vectorUint64) {
        let size = vectorUint64.size();
        let array = new BigUint64Array(size);
        for (let i = 0; i < size; ++i) {
            array[i] = vectorUint64.get(i);
        }
        return array;
    },
    toVectorUint64: function(array) {
        let vectorUint64 = new __internalPosemesh.VectorUint64();
        vectorUint64.resize(array.length);
        try {
            if (array instanceof BigUint64Array) {
                let i = 0;
                for (let item of array) {
                    vectorUint64.set(i, item);
                    i++;
                }
            } else if (array instanceof Uint32Array || array instanceof Uint16Array || array instanceof Uint8Array) {
                let i = 0;
                for (let item of array) {
                    vectorUint64.set(i, BigInt(item));
                    i++;
                }
            } else {
                let i = 0;
                for (let item of array) {
                    if (typeof item === 'bigint') {
                        if (item >= 0n && item <= 18446744073709551615n) {
                            vectorUint64.set(i, item);
                            i++;
                            continue;
                        }
                    } else if (typeof item === 'number') {
                        let bigIntItem = BigInt(item);
                        if (bigIntItem >= 0n && bigIntItem <= 18446744073709551615n) {
                            vectorUint64.set(i, bigIntItem);
                            i++;
                            continue;
                        }
                    }
                    throw new Error(`Array item at index ${i} is not an unsigned 64-bit integer.`);
                }
            }
            return vectorUint64;
        } catch (error) {
            vectorUint64.delete();
            throw error;
        }
    },

    fromVectorFloat: function(vectorFloat) {
        let size = vectorFloat.size();
        let array = new Float32Array(size);
        for (let i = 0; i < size; ++i) {
            array[i] = vectorFloat.get(i);
        }
        return array;
    },
    toVectorFloat: function(array) {
        let vectorFloat = new __internalPosemesh.VectorFloat();
        vectorFloat.resize(array.length);
        try {
            if (array instanceof Float32Array || array instanceof Float64Array || array instanceof Int32Array || array instanceof Int16Array || array instanceof Int8Array || array instanceof Uint32Array || array instanceof Uint16Array || array instanceof Uint8Array) {
                let i = 0;
                for (let item of array) {
                    vectorFloat.set(i, item);
                    i++;
                }
            } else {
                let i = 0;
                for (let item of array) {
                    if (typeof item === 'bigint') {
                        vectorFloat.set(i, Number(item));
                        i++;
                        continue;
                    } else if (typeof item === 'number') {
                        vectorFloat.set(i, item);
                        i++;
                        continue;
                    }
                    throw new Error(`Array item at index ${i} is not a number.`);
                }
            }
            return vectorFloat;
        } catch (error) {
            vectorFloat.delete();
            throw error;
        }
    },

    fromVectorDouble: function(vectorDouble) {
        let size = vectorDouble.size();
        let array = new Float64Array(size);
        for (let i = 0; i < size; ++i) {
            array[i] = vectorDouble.get(i);
        }
        return array;
    },
    toVectorDouble: function(array) {
        let vectorDouble = new __internalPosemesh.VectorDouble();
        vectorDouble.resize(array.length);
        try {
            if (array instanceof Float64Array || array instanceof Float32Array || array instanceof Int32Array || array instanceof Int16Array || array instanceof Int8Array || array instanceof Uint32Array || array instanceof Uint16Array || array instanceof Uint8Array) {
                let i = 0;
                for (let item of array) {
                    vectorDouble.set(i, item);
                    i++;
                }
            } else {
                let i = 0;
                for (let item of array) {
                    if (typeof item === 'bigint') {
                        vectorDouble.set(i, Number(item));
                        i++;
                        continue;
                    } else if (typeof item === 'number') {
                        vectorDouble.set(i, item);
                        i++;
                        continue;
                    }
                    throw new Error(`Array item at index ${i} is not a number.`);
                }
            }
            return vectorDouble;
        } catch (error) {
            vectorDouble.delete();
            throw error;
        }
    },

    fromVectorBoolean: function(vectorBoolean) {
        let size = vectorBoolean.size();
        let array = new Array(size);
        for (let i = 0; i < size; ++i) {
            array[i] = (vectorBoolean.get(i) !== 0);
        }
        return array;
    },
    toVectorBoolean: function(array) {
        let vectorBoolean = new __internalPosemesh.VectorBoolean();
        vectorBoolean.resize(array.length);
        try {
            let i = 0;
            for (let item of array) {
                if (typeof item === 'boolean') {
                    vectorBoolean.set(i, item ? 1 : 0);
                    i++;
                    continue;
                }
                throw new Error(`Array item at index ${i} is not a boolean.`);
            }
            return vectorBoolean;
        } catch (error) {
            vectorBoolean.delete();
            throw error;
        }
    },

    fromVectorString: function(vectorString) {
        let size = vectorString.size();
        let array = new Array(size);
        for (let i = 0; i < size; ++i) {
            array[i] = vectorString.get(i);
        }
        return array;
    },
    toVectorString: function(array) {
        let vectorString = new __internalPosemesh.VectorString();
        vectorString.resize(array.length);
        try {
            let i = 0;
            for (let item of array) {
                if (typeof item === 'string') {
                    vectorString.set(i, item);
                    i++;
                    continue;
                }
                throw new Error(`Array item at index ${i} is not a string.`);
            }
            return vectorString;
        } catch (error) {
            vectorString.delete();
            throw error;
        }
    }
};
