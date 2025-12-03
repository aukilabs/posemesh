#!/bin/bash
set -e

# --- Configurable via environment variables ---
CRATE_NAME="${CRATE_NAME:-posemesh-domain-http}"   # Rust crate name
CRATE_DIR="${CRATE_DIR:-.}"              # Path to Rust crate
OUT_DIR="${OUT_DIR:-pkg}"              # Output directory
TARGET="web"                          # bundler target

PACKAGE_JSON="./package.json"

# --- Step 1: Build wasm-pack ---
echo "🚀 building wasm package '$CRATE_NAME'..."
wasm-pack build "$CRATE_DIR" --target "$TARGET" --out-dir "$OUT_DIR" --release

cd $CRATE_DIR/$OUT_DIR

CRATE_NAME_SAFE="${CRATE_NAME//-/_}"

JS_FILE="${CRATE_NAME_SAFE}.js"
WASM_FILE="${CRATE_NAME_SAFE}_bg.wasm"
LOADER_FILE="index.js"

# --- Step 2: Generate universal loader ---
echo "🌍 Generating universal loader at $LOADER_FILE..."
cat > "$LOADER_FILE" <<EOL
// AUTO-GENERATED UNIVERSAL WASM LOADER
import init from "./$JS_FILE";

if (typeof window === "undefined") {
  const fs = await import("fs");
  const { dirname, resolve } = await import("path");
  const {fileURLToPath} = await import("url");
  const __filename = fileURLToPath(import.meta.url);
  const __dirname = dirname(__filename);
  const wasmPath = resolve(__dirname, "./$WASM_FILE");
  const bytes = fs.readFileSync(wasmPath);
  await init(bytes);
} else {
  await init(new URL("./$WASM_FILE", import.meta.url));
}

export * from "./$JS_FILE";
EOL

echo "✅ Universal loader ready!"

# --- Step 3: Update package.json to point main -> index.js ---
if [ -f "$PACKAGE_JSON" ]; then
    echo "📦 Updating package.json 'main' field to '$LOADER_FILE' and renaming package to '@auki/domain-client'..."
    # Use jq if available
    if command -v jq >/dev/null 2>&1; then
        jq --arg main "index.js" --arg name "@auki/domain-client" \
            '.main = $main | .files += ["index.js"] | .files |= unique | .name = $name' \
            "$PACKAGE_JSON" > tmp.json && mv tmp.json "$PACKAGE_JSON"
    else
        # fallback: simple sed (works for most cases)
        sed -i.bak 's#"main": *".*"#"main": "index.js"#' "$PACKAGE_JSON"
        # Ensure index.js is in "files"
        if ! grep -q '"index.js"' "$PACKAGE_JSON"; then
            sed -i.bak 's#\("files": *\[\)#\1"index.js", #' "$PACKAGE_JSON"
        fi
        # Replace the name field
        sed -i.bak 's#"name": *".*"#"name": "@auki/domain-client"#' "$PACKAGE_JSON"
    fi
    echo "✅ package.json updated"
fi
