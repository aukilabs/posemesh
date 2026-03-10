const path = require('path');
const glob = require('glob');
const { spawnSync } = require('child_process');

const schemaPath = path.resolve(__dirname, 'gentool.schema.json');

function runAjvOnFile(filePath) {
  const result = spawnSync(
    path.resolve(__dirname, 'node_modules', '.bin', 'ajv'),
    ['validate', '--spec=draft2020', '--all-errors', '-s', schemaPath, '-d', filePath],
    {
      stdio: 'inherit'
    }
  );
  return result.status;
}

function main() {
  const enumFiles = glob.sync(path.resolve(__dirname, '..', 'enum', '*.json'));
  const interfaceFiles = glob.sync(path.resolve(__dirname, '..', 'interface', '*.json'));

  const allFiles = [...enumFiles, ...interfaceFiles];
  if (allFiles.length === 0) {
    console.log('No enum/interface JSON files found to validate.');
    return;
  }

  let exitCode = 0;
  for (const file of allFiles) {
    const status = runAjvOnFile(file);
    if (status !== 0) {
      exitCode = status;
    }
  }

  process.exit(exitCode);
}

main();

