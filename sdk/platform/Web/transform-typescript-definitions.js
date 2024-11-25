const fs = require('fs');

const args = process.argv.slice(2);

if (args.length != 2) {
    console.error('Invalid usage.');
    process.exit(1);
    return;
}

const [inputFilePath, outputFilePath] = args;
let newLine = null;
let tab = null;

function fixConfig(content) {
    content = content.replaceAll('__getBootstraps(): VectorString;',                    'getBootstraps(): [string];');
    content = content.replaceAll('__getRelays(): VectorString;',                        'getRelays(): [string];');
    content = content.replaceAll('__setBootstraps(bootstraps: VectorString): boolean;', 'setBootstraps(bootstraps: [string]): boolean;');
    content = content.replaceAll('__setRelays(relays: VectorString): boolean;',         'setRelays(relays: [string]): boolean;');
    content = content.replaceAll('new(_0: Config): Config;',                            'new(config: Config): Config;');
    return content;
}

function fixPosemesh(content) {
    content = content.replaceAll('new(_0: Config): Posemesh;', 'new(config: Config): Posemesh;');
    content = content.replace(/ *readonly\s+__context\s*:\s*number\s*; */g, '');
    content = content.replaceAll(
        'export interface Posemesh {',
        'export interface Posemesh {' + newLine +
        tab + 'initialize(): Promise<any>;' + newLine +
        tab + 'sendMessage(message: Uint8Array, peerId: string, protocol: string): Promise<boolean>;' + newLine +
        tab + 'sendString(string: string, appendTerminatingNullCharacter: boolean, peerId: string, protocol: string): Promise<boolean>'
    );
    return content;
}

function validate(content) {
    if (content.includes('VectorString')) {
        console.error('Validation failed: Output contains \'VectorString\' string.');
        return false;
    }
    if (content.includes('__')) {
        console.error('Validation failed: Output contains \'__\' string.');
        return false;
    }
    if (content.includes('_0')) {
        console.error('Validation failed: Output contains \'_0\' string.');
        return false;
    }
    return true;
}

fs.readFile(inputFilePath, 'utf8', (error, content) => {
    if (error) {
        console.error('Failed to read input file:', error.message);
        process.exit(1);
        return;
    }

    // New line
    if (content.includes('\r\n'))
        newLine = '\r\n';
    else if (content.includes('\r'))
        newLine = '\r';
    else
        newLine = '\n';

    // Tab
    tab = content.match(/^ +/);
    if (tab) {
        tab = tab[0];
        if (tab.length <= 0)
            tab = '  ';
    } else
        tab = '  ';
    content = content.replaceAll('\t', tab);

    // Prefixes
    content = content.replace(/\/\/.*/g, '');
    content = content.replace(/declare\s+namespace\s+RuntimeExports\s*\{[\s\S]*?\}/gm, '');
    content = content.replace(/interface\s+WasmModule\s*\{[\s\S]*?\}/gm, '');
    content = content.replace(/type\s+EmbindString\s*=[\s\S]*?;/gm, '');
    content = content.replace(/export\s+interface\s+VectorString\s*\{[\s\S]*?\}/gm, '');

    // Suffixes
    content = content.replace(/export\s+type\s+MainModule\s*=[\s\S]*?;/gm, '');
    content = content.replace(/export\s+default\s+function\s+MainModuleFactory\s*\([\s\S]*?;/gm, '');
    const embindModuleContent = content.match(/interface\s+EmbindModule\s*\{ *([\s\S]*)\} */m)[1].replace(/^[\r\n]+|[\r\n]+$/g, '');
    content = content.replace(/interface\s+EmbindModule\s*\{[\s\S]*\}/gm, '');

    // Constructors
    let constructors = embindModuleContent;
    constructors = constructors.replace(/ *VectorString\s*:\s*\{[\s\S]*?\}\s*; */gm, '');
    const posemeshConstructors = ('>>>' + constructors.match(/ *Posemesh\s*:\s*\{([\s\S]*?)\}\s*; */m)[1].replace(/^[\r\n]+|[\r\n]+$/g, '')).replaceAll(newLine, newLine + '>>>').replaceAll('>>>' + tab, '>>>').replaceAll('>>>', '');
    constructors = constructors.replace(/ *Posemesh\s*:\s*\{[\s\S]*?\}\s*; */gm, '');
    constructors = constructors + posemeshConstructors;
    content = content.replace(/(export\s+interface\s+Posemesh\s*\{ *)/m, '$1' + newLine + constructors);

    // Fixes
    content = fixConfig(content);
    content = fixPosemesh(content);

    // Clean-up
    content = content.trim();
    while (content.includes(newLine + newLine))
        content = content.replaceAll(newLine + newLine, newLine);
    content = content.replaceAll('}' + newLine, '}' + newLine + newLine);
    content = content + newLine;

    fs.writeFile(outputFilePath, content, 'utf8', (error) => {
        if (error) {
            console.error('Failed to write output file:', error.message);
            process.exit(1);
            return;
        }
        if (!validate(content)) {
            process.exit(1);
            return;
        }
        console.log('Successfully transformed TypeScript definitions.');
    });
});
