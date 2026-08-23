const path = require('path');
const fs = require('fs');
const glob = require('glob');
const Ajv2020 = require('ajv/dist/2020').default;

const schemaPath = path.resolve(__dirname, 'gentool.schema.json');

/** Strip IDE-only keywords (e.g. markdownDescription) so Ajv strict mode accepts the schema. */
function stripIdeKeywords(obj) {
  if (obj === null || typeof obj !== 'object') return obj;
  if (Array.isArray(obj)) return obj.map(stripIdeKeywords);
  const result = {};
  for (const [key, value] of Object.entries(obj)) {
    if (key === 'markdownDescription') continue;
    result[key] = stripIdeKeywords(value);
  }
  return result;
}

function runAjvOnFile(validate, filePath) {
  const data = JSON.parse(fs.readFileSync(filePath, 'utf8'));
  const valid = validate(data);
  if (!valid) {
    console.error(`${filePath} invalid`);
    console.error(validate.errors);
    return 1;
  }
  console.log(`${filePath} valid`);
  return 0;
}

function main() {
  const enumFiles = glob.sync(path.resolve(__dirname, '..', 'enum', '*.json'));
  const interfaceFiles = glob.sync(path.resolve(__dirname, '..', 'interface', '*.json'));

  const allFiles = [...enumFiles, ...interfaceFiles];
  if (allFiles.length === 0) {
    console.log('No enum/interface JSON files found to validate.');
    return;
  }

  const rawSchema = JSON.parse(fs.readFileSync(schemaPath, 'utf8'));
  const schemaForAjv = stripIdeKeywords(rawSchema);

  const generatedPath = path.resolve(__dirname, 'gentool.schema.generated.json');
  if (!fs.existsSync(generatedPath)) {
    console.error('gentool.schema.generated.json not found. Run generate-schema-fragment.js or npm run generate.');
    process.exit(1);
  }
  const generatedSchema = JSON.parse(fs.readFileSync(generatedPath, 'utf8'));

  const ajv = new Ajv2020({ allErrors: true });
  ajv.addSchema(stripIdeKeywords(generatedSchema));
  const validate = ajv.compile(schemaForAjv);

  let exitCode = 0;
  for (const file of allFiles) {
    const status = runAjvOnFile(validate, file);
    if (status !== 0) {
      exitCode = status;
    }
  }

  process.exit(exitCode);
}

main();

