#!/bin/bash
# Test script for pip installation

set -e

echo "Testing LoopSleuth pip installation..."

# Create a temporary virtual environment
VENV_DIR=".test_venv"
rm -rf "$VENV_DIR"
python3 -m venv "$VENV_DIR"
source "$VENV_DIR/bin/activate"

echo "Installing build dependencies..."
pip install --upgrade pip setuptools wheel setuptools-rust

echo "Installing LoopSleuth in editable mode..."
pip install -e .

echo "Testing loopsleuth command..."
which loopsleuth
loopsleuth --help | head -10

echo "Testing Python module import..."
python -c "from loopsleuth import __version__, get_binary_path; print(f'Version: {__version__}'); print(f'Binary: {get_binary_path()}')"

echo ""
echo "✅ Installation test passed!"
echo ""
echo "To use the installed version:"
echo "  source $VENV_DIR/bin/activate"
echo "  loopsleuth -m models/model.gguf ./src"
echo ""
echo "To clean up:"
echo "  deactivate"
echo "  rm -rf $VENV_DIR"
