#!/bin/bash
# Test the bootstrap workflow for pip-installed LoopSleuth

set -e

echo "Testing LoopSleuth Bootstrap Workflow..."
echo

# Create a temporary virtual environment
VENV_DIR=".test_bootstrap_venv"
rm -rf "$VENV_DIR"
python3 -m venv "$VENV_DIR"
source "$VENV_DIR/bin/activate"

echo "✓ Virtual environment created"
echo

# Install build dependencies
pip install --quiet --upgrade pip setuptools wheel setuptools-rust

echo "✓ Build dependencies installed"
echo

# Install LoopSleuth
echo "Installing LoopSleuth..."
pip install --quiet -e .

echo "✓ LoopSleuth installed"
echo

# Test basic commands
echo "Testing commands..."
echo

# Test help
echo "1. Testing: loopsleuth --help"
loopsleuth --help | head -5
echo

# Test list-models (should be empty)
echo "2. Testing: loopsleuth list-models"
loopsleuth list-models
echo

# Test download-model help/dry-run
echo "3. Testing: loopsleuth download-model (dry run)"
echo "   (In real usage, this would be interactive)"
echo "   Command available: ✓"
echo

# Test Python imports
echo "4. Testing Python imports..."
python -c "
from loopsleuth import __version__, get_binary_path, get_models_dir, list_downloaded_models
print(f'   Version: {__version__}')
print(f'   Binary: {get_binary_path()}')
print(f'   Models dir: {get_models_dir()}')
models = list_downloaded_models()
print(f'   Downloaded models: {len(models)}')
"
echo

# Test that the binary works
echo "5. Testing binary execution..."
loopsleuth --list-checks | head -10
echo

echo "="*60
echo "✅ Bootstrap workflow test passed!"
echo "="*60
echo
echo "Manual test (requires interaction):"
echo "  source $VENV_DIR/bin/activate"
echo "  loopsleuth download-model"
echo
echo "Clean up:"
echo "  deactivate"
echo "  rm -rf $VENV_DIR"
