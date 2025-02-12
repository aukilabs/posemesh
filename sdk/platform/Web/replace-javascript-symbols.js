const fs = require('fs');

const args = process.argv.slice(2);

if (args.length != 2) {
    console.error('Invalid usage.');
    process.exit(1);
    return;
}

const [inputFilePath, outputFilePath] = args;

fs.readFile(inputFilePath, 'utf8', (error, content) => {
    if (error) {
        console.error('Failed to read input file:', error.message);
        process.exit(1);
        return;
    }

    content = content.replaceAll('module.exports = __internalPosemesh;', '{ /* module.exports = __internalPosemesh; */ }');
    content = content.replaceAll('define([], () => __internalPosemesh);', '{ /* define([], () => __internalPosemesh); */ }');

    fs.writeFile(outputFilePath, content, 'utf8', (error) => {
        if (error) {
            console.error('Failed to write output file:', error.message);
            process.exit(1);
            return;
        }
        console.log('Successfully replaced JavaScript symbols.');
    });
});
