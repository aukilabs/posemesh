(function (root, factory) {
    if (typeof define === 'function' && define.amd) {
        define('Posemesh', [], factory); // AMD (RequireJS)
    } else if (typeof module === 'object' && module.exports) {
        module.exports = factory(); // CommonJS / Node.js
    } else if (typeof exports === 'object') {
        exports.Posemesh = factory(); // CommonJS-like environments that support exports (but not module.exports)
    } else {
        root.Posemesh = factory(); // Browser (global variable)
    }
}(typeof globalThis !== 'undefined' ? globalThis : (typeof window !== 'undefined' ? window : (typeof self !== 'undefined' ? self : (typeof global !== 'undefined' ? global : this))), function() {
    let posemeshModule = {};
    let posemeshModuleCanInitialize = true;
    /// --- BEGIN MAIN JS WRAPPER --- ///
