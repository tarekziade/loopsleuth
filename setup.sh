#!/bin/bash

set -e

echo "=== LoopSleuth Setup ==="
echo

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed"
    echo "   Install from: https://rustup.rs/"
    exit 1
fi
echo "✓ Rust is installed"

# Check for CMake
if ! command -v cmake &> /dev/null; then
    echo "❌ CMake is not installed"
    echo "   Install with: brew install cmake (macOS) or apt-get install cmake (Linux)"
    exit 1
fi
echo "✓ CMake is installed"

# Check for Hugging Face CLI (optional)
if ! command -v hf &> /dev/null; then
    echo "⚠️  hf CLI is not installed (optional, for model download)"
    echo "   Install with: pip install -U huggingface_hub"
    echo "   Or use standalone: curl -LsSf https://hf.co/cli/install.sh | bash"
    HF_AVAILABLE=false
else
    echo "✓ hf CLI is installed"
    HF_AVAILABLE=true
fi

if [ "$HF_AVAILABLE" = false ]; then
    echo
    echo "⚠️  Cannot download models without hf CLI"
    echo "   Please install it first and run this script again"
    echo "   Or manually download a model to ./models/"
    MODEL_PATH=""
else
    echo
    echo "Choose a model to download:"
    echo "1. Qwen2.5-Coder (3B) - Recommended for code (~2GB) ⭐"
    echo "2. Devstral Small 2 (24B) - Best accuracy (~15GB)"
    echo "3. Qwen2.5 (3B) - General purpose (~2GB)"
    echo "4. Qwen2.5 (0.5B) - Fast (~400MB)"
    echo "5. Skip model download"
    echo
    read -p "Enter choice (1-5) [1]: " choice
    choice=${choice:-1}

    mkdir -p models

    case $choice in
        1)
            echo "Downloading Qwen2.5-Coder 3B..."
            hf download Qwen/Qwen2.5-Coder-3B-Instruct-GGUF \
                qwen2.5-coder-3b-instruct-q4_k_m.gguf \
                --local-dir ./models
            MODEL_PATH="./models/qwen2.5-coder-3b-instruct-q4_k_m.gguf"
            ;;
        2)
            echo "Downloading Devstral Small 2..."
            hf download unsloth/Devstral-Small-2-24B-Instruct-2512-GGUF \
                Devstral-Small-2-24B-Instruct-2512-Q4_K_M.gguf \
                --local-dir ./models
            MODEL_PATH="./models/Devstral-Small-2-24B-Instruct-2512-Q4_K_M.gguf"
            ;;
        3)
            echo "Downloading Qwen2.5 3B..."
            hf download Qwen/Qwen2.5-3B-Instruct-GGUF \
                qwen2.5-3b-instruct-q4_k_m.gguf \
                --local-dir ./models
            MODEL_PATH="./models/qwen2.5-3b-instruct-q4_k_m.gguf"
            ;;
        4)
            echo "Downloading Qwen2.5 0.5B..."
            hf download Qwen/Qwen2.5-0.5B-Instruct-GGUF \
                qwen2.5-0.5b-instruct-q4_k_m.gguf \
                --local-dir ./models
            MODEL_PATH="./models/qwen2.5-0.5b-instruct-q4_k_m.gguf"
            ;;
        5)
            echo "Skipping model download"
            MODEL_PATH=""
            ;;
        *)
            echo "Invalid choice"
            exit 1
            ;;
    esac
fi

echo
echo "Building LoopSleuth (this may take several minutes on first build)..."
cargo build --release

echo
echo "✓ Setup complete!"
echo
if [ -n "$MODEL_PATH" ]; then
    echo "Run analysis with:"
    echo "  ./target/release/loopsleuth --model $MODEL_PATH <path_to_python_code>"
else
    echo "Download a model first, then run:"
    echo "  ./target/release/loopsleuth --model ./models/<model_file>.gguf <path_to_python_code>"
fi
echo
echo "Example:"
echo "  ./target/release/loopsleuth --model $MODEL_PATH ./tests/checks/quadratic.py"
