.PHONY: help build release test clean run example check

help:
	@echo "LoopSleuth - Python Quadratic Complexity Detector"
	@echo ""
	@echo "Available targets:"
	@echo "  make build    - Build debug version"
	@echo "  make release  - Build optimized release version"
	@echo "  make test     - Run tests"
	@echo "  make clean    - Clean build artifacts"
	@echo "  make check    - Check code without building"
	@echo "  make example  - Run example analysis (requires model)"
	@echo "  make setup    - Run setup script"

setup:
	./setup.sh

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
	@if [ -f "./models/qwen2.5-coder-3b-instruct-q4_k_m.gguf" ]; then \
		./target/release/loopsleuth --model ./models/qwen2.5-coder-3b-instruct-q4_k_m.gguf ./test_examples/sample.py; \
	elif [ -f "./models/qwen2.5-3b-instruct-q4_k_m.gguf" ]; then \
		./target/release/loopsleuth --model ./models/qwen2.5-3b-instruct-q4_k_m.gguf ./test_examples/sample.py; \
	elif [ -f "./models/qwen2.5-0.5b-instruct-q4_k_m.gguf" ]; then \
		./target/release/loopsleuth --model ./models/qwen2.5-0.5b-instruct-q4_k_m.gguf ./test_examples/sample.py; \
	elif [ -f "./models/Devstral-Small-2-24B-Instruct-2512-Q4_K_M.gguf" ]; then \
		./target/release/loopsleuth --model ./models/Devstral-Small-2-24B-Instruct-2512-Q4_K_M.gguf ./test_examples/sample.py; \
	else \
		echo "No model found in ./models/"; \
		echo "Run 'make setup' to download a model first"; \
		exit 1; \
	fi

run: release
	@echo "Usage: make run MODEL=<path> PATH=<python_path>"
	@if [ -z "$(MODEL)" ] || [ -z "$(PATH)" ]; then \
		echo "Example: make run MODEL=./models/model.gguf PATH=./test_examples/"; \
		exit 1; \
	fi
	./target/release/loopsleuth --model $(MODEL) $(PATH)
