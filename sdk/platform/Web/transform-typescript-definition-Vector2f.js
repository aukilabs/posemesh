/* This code is automatically generated from Vector2f.json interface. Do not modify it manually as it will be overwritten! */

function fixVector2f(content, newLine, tab) {
    // Find member area
    const memberAreaMatches = content.match(/export interface Vector2f \{(.|\r|\n)*?\}/gm);
    if (!Array.isArray(memberAreaMatches) || memberAreaMatches.length === 0) {
        throw new Error('Member area for \'Vector2f\' not found.');
    }
    if (memberAreaMatches.length > 1) {
        throw new Error('Multiple member areas for \'Vector2f\' found.');
    }
    const memberArea = memberAreaMatches[0];

    // Find static area
    const staticAreaMatches = content.match(/Vector2f: \{(.|\r|\n)*?\};/gm);
    if (!Array.isArray(staticAreaMatches) || staticAreaMatches.length === 0) {
        throw new Error('Static area for \'Vector2f\' not found.');
    }
    if (staticAreaMatches.length > 1) {
        throw new Error('Multiple static areas for \'Vector2f\' found.');
    }
    const staticArea = staticAreaMatches[0];

    return content;
}

module.exports = fixVector2f;
