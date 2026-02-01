#!/usr/bin/env python3
"""Setup script for LoopSleuth Python package."""

from setuptools import setup
from setuptools_rust import Binding, RustExtension

setup(
    rust_extensions=[
        RustExtension(
            "loopsleuth.loopsleuth_bin",
            binding=Binding.Exec,
            strip=True,
        )
    ],
    zip_safe=False,
)
