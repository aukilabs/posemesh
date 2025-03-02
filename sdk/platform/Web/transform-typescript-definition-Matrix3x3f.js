/* This code is automatically generated from Matrix3x3f.json interface. Do not modify it manually as it will be overwritten! */

function fixMatrix3x3f(content, newLine, tab) {
    // Find member area
    const memberAreaMatches = content.match(/export interface Matrix3x3f \{(.|\r|\n)*?\}/gm);
    if (!Array.isArray(memberAreaMatches) || memberAreaMatches.length === 0) {
        throw new Error('Member area for \'Matrix3x3f\' not found.');
    }
    if (memberAreaMatches.length > 1) {
        throw new Error('Multiple member areas for \'Matrix3x3f\' found.');
    }
    const memberArea = memberAreaMatches[0];

    // Find static area
    const staticAreaMatches = content.match(/Matrix3x3f: \{(.|\r|\n)*?\};/gm);
    if (!Array.isArray(staticAreaMatches) || staticAreaMatches.length === 0) {
        throw new Error('Static area for \'Matrix3x3f\' not found.');
    }
    if (staticAreaMatches.length > 1) {
        throw new Error('Multiple static areas for \'Matrix3x3f\' found.');
    }
    const staticArea = staticAreaMatches[0];

    return content;
}

module.exports = fixMatrix3x3f;
