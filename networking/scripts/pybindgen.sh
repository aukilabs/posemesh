#!/usr/bin/env sh

python3 -m venv .pyvenv
source .pyvenv/bin/activate
python3 -m pip install maturin

# Build the Python bindings
maturin build -r --bindings pyo3 --features "py"
