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
    echo "Choose inference mode:"
    echo
    echo "  LOCAL (download a GGUF model):"
    echo "  1. Qwen2.5-Coder (7B) - Recommended for code (~4.7GB) ⭐"
    echo "  2. Qwen2.5-Coder (3B) - Faster but less accurate (~2GB)"
    echo "  3. Devstral Small 2 (24B) - Best accuracy (~15GB)"
    echo "  4. Qwen2.5 (3B) - General purpose (~2GB)"
    echo "  5. Qwen2.5 (0.5B) - Fast (~400MB)"
    echo "  6. Qwen3.5 (2B) - Compact newer general-purpose alternative (~1.3GB)"
    echo "  7. Qwen3.5 (4B) - Stronger compact alternative (~3GB)"
    echo "  8. Gemma 4 (E2B) - Strong alternative for local analysis (~3.1GB)"
    echo
    echo "  SERVER-SIDE (use a HF Inference Endpoint):"
    echo "  9. Configure a HF Inference Endpoint (no local GPU needed)"
    echo
    echo "  0. Skip model setup"
    echo
    read -p "Enter choice (0-9) [1]: " choice
    choice=${choice:-1}

    mkdir -p models

    case $choice in
        1)
            echo "Downloading Qwen2.5-Coder 7B..."
            hf download unsloth/Qwen2.5-Coder-7B-Instruct-128K-GGUF \
                Qwen2.5-Coder-7B-Instruct-128K-Q4_K_M.gguf \
                --local-dir ./models
            MODEL_PATH="./models/Qwen2.5-Coder-7B-Instruct-128K-Q4_K_M.gguf"
            ;;
        2)
            echo "Downloading Qwen2.5-Coder 3B..."
            hf download Qwen/Qwen2.5-Coder-3B-Instruct-GGUF \
                qwen2.5-coder-3b-instruct-q4_k_m.gguf \
                --local-dir ./models
            MODEL_PATH="./models/qwen2.5-coder-3b-instruct-q4_k_m.gguf"
            ;;
        3)
            echo "Downloading Devstral Small 2..."
            hf download unsloth/Devstral-Small-2-24B-Instruct-2512-GGUF \
                Devstral-Small-2-24B-Instruct-2512-Q4_K_M.gguf \
                --local-dir ./models
            MODEL_PATH="./models/Devstral-Small-2-24B-Instruct-2512-Q4_K_M.gguf"
            ;;
        4)
            echo "Downloading Qwen2.5 3B..."
            hf download Qwen/Qwen2.5-3B-Instruct-GGUF \
                qwen2.5-3b-instruct-q4_k_m.gguf \
                --local-dir ./models
            MODEL_PATH="./models/qwen2.5-3b-instruct-q4_k_m.gguf"
            ;;
        5)
            echo "Downloading Qwen2.5 0.5B..."
            hf download Qwen/Qwen2.5-0.5B-Instruct-GGUF \
                qwen2.5-0.5b-instruct-q4_k_m.gguf \
                --local-dir ./models
            MODEL_PATH="./models/qwen2.5-0.5b-instruct-q4_k_m.gguf"
            ;;
        6)
            echo "Downloading Qwen3.5 2B..."
            hf download unsloth/Qwen3.5-2B-GGUF \
                Qwen3.5-2B-Q4_K_M.gguf \
                --local-dir ./models
            MODEL_PATH="./models/Qwen3.5-2B-Q4_K_M.gguf"
            ;;
        7)
            echo "Downloading Qwen3.5 4B..."
            hf download unsloth/Qwen3.5-4B-GGUF \
                Qwen3.5-4B-Q4_K_M.gguf \
                --local-dir ./models
            MODEL_PATH="./models/Qwen3.5-4B-Q4_K_M.gguf"
            ;;
        8)
            echo "Downloading Gemma 4 E2B..."
            hf download unsloth/gemma-4-E2B-it-GGUF \
                gemma-4-E2B-it-Q4_K_M.gguf \
                --local-dir ./models
            MODEL_PATH="./models/gemma-4-E2B-it-Q4_K_M.gguf"
            ;;
        9)
            echo
            echo "=== HF Inference Endpoint Setup ==="
            echo
            echo "You need:"
            echo "  1. A HF Inference Endpoint URL (from https://ui.endpoints.huggingface.co)"
            echo "  2. A HF token with access to the endpoint"
            echo
            read -p "Enter your endpoint URL: " API_URL
            if [ -z "$API_URL" ]; then
                echo "❌ No URL provided, skipping"
                MODEL_PATH=""
            else
                read -p "Enter your HF token (hf_...): " HF_TOKEN_INPUT
                echo
                echo "Testing endpoint..."
                if curl -sf -o /dev/null -w "%{http_code}" \
                    -H "Authorization: Bearer $HF_TOKEN_INPUT" \
                    "$API_URL/v1/models" | grep -q "200"; then
                    echo "✅ Endpoint is reachable!"
                    MODEL_NAME=$(curl -s -H "Authorization: Bearer $HF_TOKEN_INPUT" \
                        "$API_URL/v1/models" | python3 -c "import sys,json; print(json.load(sys.stdin)['data'][0]['id'])" 2>/dev/null || echo "unknown")
                    echo "   Model: $MODEL_NAME"
                else
                    echo "⚠️  Could not reach endpoint (it may be sleeping). Continuing anyway."
                fi
                echo
                echo "Add this to your shell profile (~/.bashrc or ~/.zshrc):"
                echo "  export HF_TOKEN=\"$HF_TOKEN_INPUT\""
                echo
                echo "Run analysis with:"
                echo "  HF_TOKEN=\"$HF_TOKEN_INPUT\" ./target/release/loopsleuth_bin --api-url $API_URL <path_to_python_code>"
                MODEL_PATH=""
                API_URL_CONFIGURED="$API_URL"
                API_TOKEN_CONFIGURED="$HF_TOKEN_INPUT"
            fi
            ;;
        0)
            echo "Skipping model setup"
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
    echo "  ./target/release/loopsleuth_bin --model $MODEL_PATH <path_to_python_code>"
    echo
    echo "Example:"
    echo "  ./target/release/loopsleuth_bin --model $MODEL_PATH ./tests/checks/quadratic.py"
elif [ -n "${API_URL_CONFIGURED:-}" ]; then
    echo "Run analysis with:"
    echo "  HF_TOKEN=\"$API_TOKEN_CONFIGURED\" ./target/release/loopsleuth_bin --api-url $API_URL_CONFIGURED <path_to_python_code>"
    echo
    echo "Example:"
    echo "  HF_TOKEN=\"$API_TOKEN_CONFIGURED\" ./target/release/loopsleuth_bin --api-url $API_URL_CONFIGURED ./tests/checks/quadratic.py"
else
    echo "To use a local model, download one and run:"
    echo "  ./target/release/loopsleuth_bin --model ./models/<model_file>.gguf <path_to_python_code>"
    echo
    echo "To use a HF Inference Endpoint instead:"
    echo "  HF_TOKEN=\"hf_...\" ./target/release/loopsleuth_bin --api-url https://your-endpoint.aws.endpoints.huggingface.cloud <path_to_python_code>"
fi
