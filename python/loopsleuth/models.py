"""Model management for LoopSleuth."""

import os
import sys
from pathlib import Path
from typing import Optional


# Model definitions
MODELS = {
    "1": {
        "name": "Qwen2.5-Coder (3B) - Recommended ⭐",
        "repo": "Qwen/Qwen2.5-Coder-3B-Instruct-GGUF",
        "filename": "qwen2.5-coder-3b-instruct-q4_k_m.gguf",
        "size": "~2GB",
        "description": "Best for code analysis, excellent accuracy",
    },
    "2": {
        "name": "Devstral Small 2 (24B)",
        "repo": "unsloth/Devstral-Small-2-24B-Instruct-2512-GGUF",
        "filename": "Devstral-Small-2-24B-Instruct-2512-Q4_K_M.gguf",
        "size": "~15GB",
        "description": "Highest accuracy, requires more RAM",
    },
    "3": {
        "name": "Qwen2.5 (3B)",
        "repo": "Qwen/Qwen2.5-3B-Instruct-GGUF",
        "filename": "qwen2.5-3b-instruct-q4_k_m.gguf",
        "size": "~2GB",
        "description": "General purpose, good balance",
    },
    "4": {
        "name": "Qwen2.5 (0.5B)",
        "repo": "Qwen/Qwen2.5-0.5B-Instruct-GGUF",
        "filename": "qwen2.5-0.5b-instruct-q4_k_m.gguf",
        "size": "~400MB",
        "description": "Very fast, lower accuracy",
    },
}


def get_models_dir() -> Path:
    """Get the default models directory.

    Priority:
    1. LOOPSLEUTH_MODELS_DIR environment variable
    2. ~/.loopsleuth/models/
    """
    if "LOOPSLEUTH_MODELS_DIR" in os.environ:
        return Path(os.environ["LOOPSLEUTH_MODELS_DIR"])

    return Path.home() / ".loopsleuth" / "models"


def ensure_models_dir() -> Path:
    """Ensure the models directory exists and return its path."""
    models_dir = get_models_dir()
    models_dir.mkdir(parents=True, exist_ok=True)
    return models_dir


def list_downloaded_models() -> list[tuple[str, Path]]:
    """List all downloaded GGUF models in the models directory."""
    models_dir = get_models_dir()
    if not models_dir.exists():
        return []

    models = []
    for file in models_dir.glob("*.gguf"):
        size_mb = file.stat().st_size / (1024 * 1024)
        models.append((f"{file.name} ({size_mb:.1f} MB)", file))

    return sorted(models)


def print_model_menu():
    """Print the interactive model selection menu."""
    print("\n╔═══════════════════════════════════════════════════════════╗")
    print("║           LoopSleuth Model Download                      ║")
    print("╚═══════════════════════════════════════════════════════════╝\n")

    print("Choose a model to download:\n")

    for key, model in MODELS.items():
        print(f"{key}. {model['name']} ({model['size']})")
        print(f"   {model['description']}\n")

    print("0. Exit without downloading\n")


def download_model(choice: str, models_dir: Path) -> Optional[Path]:
    """Download the selected model using huggingface_hub.

    Args:
        choice: User's choice (1-4)
        models_dir: Directory to download to

    Returns:
        Path to downloaded model, or None if cancelled/failed
    """
    if choice not in MODELS:
        print(f"❌ Invalid choice: {choice}")
        return None

    model = MODELS[choice]

    try:
        from huggingface_hub import hf_hub_download
    except ImportError:
        print("\n❌ huggingface_hub is not installed")
        print("   Install with: pip install huggingface_hub")
        print("\n   Or use the hf CLI:")
        print(f"   hf download {model['repo']} {model['filename']} --local-dir {models_dir}")
        return None

    print(f"\n📥 Downloading {model['name']}...")
    print(f"   Repository: {model['repo']}")
    print(f"   File: {model['filename']}")
    print(f"   Size: {model['size']}")
    print(f"   Destination: {models_dir}\n")
    print("   This may take several minutes depending on your connection...\n")

    try:
        # Download the model
        downloaded_path = hf_hub_download(
            repo_id=model['repo'],
            filename=model['filename'],
            local_dir=models_dir,
            local_dir_use_symlinks=False,  # Copy instead of symlink
        )

        model_path = Path(downloaded_path)

        print(f"\n✅ Download complete!")
        print(f"   Model saved to: {model_path}")

        return model_path

    except Exception as e:
        print(f"\n❌ Download failed: {e}")
        print("\n   You can try downloading manually:")
        print(f"   hf download {model['repo']} {model['filename']} --local-dir {models_dir}")
        return None


def interactive_download() -> Optional[Path]:
    """Run interactive model download process.

    Returns:
        Path to downloaded model, or None if cancelled/failed
    """
    models_dir = ensure_models_dir()

    # Show existing models
    existing = list_downloaded_models()
    if existing:
        print("\n📦 Already downloaded models:")
        for name, path in existing:
            print(f"   • {name}")
            print(f"     {path}")
        print()

    # Show menu
    print_model_menu()

    # Get user choice
    try:
        choice = input("Enter choice (1-4, 0 to exit) [1]: ").strip()
        if not choice:
            choice = "1"  # Default to recommended model

        if choice == "0":
            print("\n👋 Cancelled")
            return None

        return download_model(choice, models_dir)

    except KeyboardInterrupt:
        print("\n\n👋 Cancelled")
        return None
    except EOFError:
        print("\n\n👋 Cancelled")
        return None


def print_usage_instructions(model_path: Optional[Path] = None):
    """Print instructions for using LoopSleuth after setup."""
    print("\n" + "="*60)
    print("  LoopSleuth is ready! 🎉")
    print("="*60 + "\n")

    if model_path:
        print("Run analysis with:")
        print(f"  loopsleuth -m {model_path} <path_to_python_code>\n")
        print("Example:")
        print(f"  loopsleuth -m {model_path} ./my_project/\n")
    else:
        models_dir = get_models_dir()
        existing = list_downloaded_models()

        if existing:
            name, path = existing[0]
            print("Run analysis with one of your downloaded models:")
            print(f"  loopsleuth -m {path} <path_to_python_code>\n")
        else:
            print("Download a model first:")
            print("  loopsleuth download-model\n")
            print("Then run analysis:")
            print(f"  loopsleuth -m {models_dir}/<model>.gguf <path_to_python_code>\n")

    print("Other commands:")
    print("  loopsleuth --list-checks          # Show all available checks")
    print("  loopsleuth --help                 # Show all options")
    print("  loopsleuth download-model         # Download additional models")
    print()


def main():
    """Entry point for the download-model command."""
    print("\n🔍 LoopSleuth - AI-powered Python performance analyzer\n")

    # Check if huggingface_hub is available
    try:
        import huggingface_hub
    except ImportError:
        print("⚠️  huggingface_hub is not installed")
        print("\n   Installing huggingface_hub...")
        try:
            import subprocess
            subprocess.check_call([sys.executable, "-m", "pip", "install", "-q", "huggingface_hub"])
            print("   ✅ Installation complete!\n")
        except Exception as e:
            print(f"   ❌ Installation failed: {e}")
            print("\n   Please install manually:")
            print("   pip install huggingface_hub")
            return 1

    # Run interactive download
    model_path = interactive_download()

    # Print usage instructions
    print_usage_instructions(model_path)

    return 0


if __name__ == "__main__":
    sys.exit(main())
