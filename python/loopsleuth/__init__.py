"""LoopSleuth - AI-powered Python performance analyzer.

Detects algorithmic bottlenecks in Python code using local LLM analysis.
"""

__version__ = "0.1.0"

from pathlib import Path

# Path to the bundled binary
BINARY_NAME = "loopsleuth_bin"

def get_binary_path():
    """Get the path to the bundled loopsleuth binary."""
    import sys
    import os

    # The binary will be installed alongside the Python package
    package_dir = Path(__file__).parent

    # On Windows, executables have .exe extension
    if sys.platform == "win32":
        binary_name = f"{BINARY_NAME}.exe"
    else:
        binary_name = BINARY_NAME

    binary_path = package_dir / binary_name

    if not binary_path.exists():
        raise RuntimeError(
            f"LoopSleuth binary not found at {binary_path}. "
            "Installation may have failed."
        )

    return str(binary_path)

from .models import get_models_dir, list_downloaded_models

__all__ = ["__version__", "get_binary_path", "get_models_dir", "list_downloaded_models"]
