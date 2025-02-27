async function __internalPosemeshBase(source) {
    if (source instanceof URL) {
        source = source.href;
    }
    if (typeof source === 'string') {
        if (typeof window === 'undefined') {
            let fsPromises = undefined;
            try {
                fsPromises = require('fs/promises');
            } catch {
                fsPromises = require('fs').promises;
            }
            source = await fsPromises.readFile(source);
        } else {
            if (typeof fetch !== 'function') {
                throw new Error('Unavailable \'fetch()\' function.');
            }
            source = fetch(source);
        }
    }

    /// --- BEGIN RUST JS WRAPPER --- ///
