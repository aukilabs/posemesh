#!/usr/bin/env sh

python -m venv .pyvenv
source .pyvenv/bin/activate
python -m pip install maturin

# Build the Python bindings
maturin build -r --bindings pyo3 --features "py"
