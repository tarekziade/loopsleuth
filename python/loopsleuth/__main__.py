"""CLI entry point for LoopSleuth."""

import sys
import subprocess
from . import get_binary_path


def main():
    """Run the LoopSleuth binary with provided arguments."""
    # Check for subcommands
    args = sys.argv[1:]

    # Handle download-model subcommand
    if args and args[0] in ("download-model", "download"):
        from .models import main as download_main
        sys.exit(download_main())

    # Handle list-models subcommand
    if args and args[0] == "list-models":
        from .models import list_downloaded_models, get_models_dir
        models_dir = get_models_dir()
        print(f"\n📦 Downloaded models in {models_dir}:\n")
        models = list_downloaded_models()
        if models:
            for name, path in models:
                print(f"   • {name}")
                print(f"     {path}")
            print()
        else:
            print("   No models found.")
            print(f"   Download a model with: loopsleuth download-model\n")
        sys.exit(0)

    # Handle help for subcommands
    if args and args[0] in ("-h", "--help") and len(args) == 1:
        print_main_help()
        sys.exit(0)

    # Otherwise, run the Rust binary
    try:
        binary_path = get_binary_path()
    except RuntimeError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    try:
        # Run the binary and pass through stdin/stdout/stderr
        result = subprocess.run(
            [binary_path] + args,
            check=False,  # Don't raise exception on non-zero exit
        )
        sys.exit(result.returncode)
    except FileNotFoundError:
        print(f"Error: Could not execute {binary_path}", file=sys.stderr)
        sys.exit(1)
    except KeyboardInterrupt:
        sys.exit(130)  # Standard exit code for Ctrl+C


def print_main_help():
    """Print help message including Python subcommands."""
    from . import __version__

    print(f"""
LoopSleuth v{__version__} - AI-powered Python performance analyzer

USAGE:
    loopsleuth [SUBCOMMAND | OPTIONS]

SUBCOMMANDS:
    download-model    Download a model from Hugging Face interactively
    list-models       List all downloaded models

For analysis options, run:
    loopsleuth --help

EXAMPLES:
    # First time setup - download a model
    loopsleuth download-model

    # List downloaded models
    loopsleuth list-models

    # Run analysis
    loopsleuth -m ~/.loopsleuth/models/qwen2.5-coder-3b-instruct-q4_k_m.gguf ./src

    # Show all checks
    loopsleuth --list-checks
""")


if __name__ == "__main__":
    main()
