/* This code is automatically generated from Matrix4x4f.json interface. Do not modify it manually as it will be overwritten! */

function fixMatrix4x4f(content, newLine, tab) {
    // Find member area
    const memberAreaMatches = content.match(/export interface Matrix4x4f \{(.|\r|\n)*?\}/gm);
    if (!Array.isArray(memberAreaMatches) || memberAreaMatches.length === 0) {
        throw new Error('Member area for \'Matrix4x4f\' not found.');
    }
    if (memberAreaMatches.length > 1) {
        throw new Error('Multiple member areas for \'Matrix4x4f\' found.');
    }
    const memberArea = memberAreaMatches[0];

    // Find static area
    const staticAreaMatches = content.match(/Matrix4x4f: \{(.|\r|\n)*?\};/gm);
    if (!Array.isArray(staticAreaMatches) || staticAreaMatches.length === 0) {
        throw new Error('Static area for \'Matrix4x4f\' not found.');
    }
    if (staticAreaMatches.length > 1) {
        throw new Error('Multiple static areas for \'Matrix4x4f\' found.');
    }
    const staticArea = staticAreaMatches[0];

    return content;
}

module.exports = fixMatrix4x4f;
