.PHONY: help build release test clean run example check install

help:
	@echo "LoopSleuth - Python Performance Analyzer"
	@echo ""
	@echo "Quick Start (Recommended):"
	@echo "  pip install loopsleuth"
	@echo "  loopsleuth download-model"
	@echo "  loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./src"
	@echo ""
	@echo "Development targets:"
	@echo "  make install  - Install in development mode"
	@echo "  make build    - Build debug version"
	@echo "  make release  - Build optimized release version"
	@echo "  make test     - Run tests"
	@echo "  make clean    - Clean build artifacts"
	@echo "  make check    - Check code without building"
	@echo "  make example  - Run example analysis (requires model)"

install:
	pip install -e .

build:
	cargo build

release:
	cargo build --release

test:
	cargo test

check:
	cargo check

clean:
	cargo clean

example: release
	@if [ -f "$(HOME)/.loopsleuth/models/qwen2.5-coder-3b-instruct-q4_k_m.gguf" ]; then \
		./target/release/loopsleuth --model $(HOME)/.loopsleuth/models/qwen2.5-coder-3b-instruct-q4_k_m.gguf ./test_examples/sample.py; \
	elif [ -f "./models/qwen2.5-coder-3b-instruct-q4_k_m.gguf" ]; then \
		./target/release/loopsleuth --model ./models/qwen2.5-coder-3b-instruct-q4_k_m.gguf ./test_examples/sample.py; \
	elif [ -f "$(HOME)/.loopsleuth/models/qwen2.5-3b-instruct-q4_k_m.gguf" ]; then \
		./target/release/loopsleuth --model $(HOME)/.loopsleuth/models/qwen2.5-3b-instruct-q4_k_m.gguf ./test_examples/sample.py; \
	elif [ -f "./models/qwen2.5-3b-instruct-q4_k_m.gguf" ]; then \
		./target/release/loopsleuth --model ./models/qwen2.5-3b-instruct-q4_k_m.gguf ./test_examples/sample.py; \
	else \
		echo "No model found in ~/.loopsleuth/models/ or ./models/"; \
		echo "Run 'loopsleuth download-model' to download a model"; \
		exit 1; \
	fi

run: release
	@echo "Usage: make run MODEL=<path> PATH=<python_path>"
	@if [ -z "$(MODEL)" ] || [ -z "$(PATH)" ]; then \
		echo "Example: make run MODEL=./models/model.gguf PATH=./test_examples/"; \
		exit 1; \
	fi
	./target/release/loopsleuth --model $(MODEL) $(PATH)
