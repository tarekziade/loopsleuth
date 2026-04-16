.PHONY: help install build release test check clean fmt fmt-check clippy \
	run example \
	golden-update golden-verify test-regression test-bootstrap test-pip-install lint-yaml

EXAMPLE_MODEL_CANDIDATES = \
	$(HOME)/.loopsleuth/models/Qwen2.5-Coder-7B-Instruct-128K-Q4_K_M.gguf \
	./models/Qwen2.5-Coder-7B-Instruct-128K-Q4_K_M.gguf \
	$(HOME)/.loopsleuth/models/Qwen3.5-4B-Q4_K_M.gguf \
	./models/Qwen3.5-4B-Q4_K_M.gguf \
	$(HOME)/.loopsleuth/models/Qwen3.5-2B-Q4_K_M.gguf \
	./models/Qwen3.5-2B-Q4_K_M.gguf \
	$(HOME)/.loopsleuth/models/gemma-4-E2B-it-Q4_K_M.gguf \
	./models/gemma-4-E2B-it-Q4_K_M.gguf \
	$(HOME)/.loopsleuth/models/qwen2.5-coder-3b-instruct-q4_k_m.gguf \
	./models/qwen2.5-coder-3b-instruct-q4_k_m.gguf \
	$(HOME)/.loopsleuth/models/qwen2.5-3b-instruct-q4_k_m.gguf \
	./models/qwen2.5-3b-instruct-q4_k_m.gguf

GOLDEN_TEST_MODEL ?= $(HOME)/.loopsleuth/models/Qwen2.5-Coder-7B-Instruct-128K-Q4_K_M.gguf

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
	@echo "  make check    - Check code without building"
	@echo "  make test     - Run Rust tests"
	@echo "  make clean    - Clean build artifacts"
	@echo "  make fmt      - Format Rust code"
	@echo "  make fmt-check - Check Rust formatting"
	@echo "  make clippy   - Run clippy lints"
	@echo "  make example  - Run example analysis (requires model)"
	@echo ""
	@echo "Golden tests:"
	@echo "  make golden-update - Generate/update golden files"
	@echo "  make golden-verify - Verify against golden files"
	@echo ""
	@echo "Other tests:"
	@echo "  make test-regression - Run golden test wrapper"
	@echo "  make test-bootstrap  - Run bootstrap checks"
	@echo "  make test-pip-install - Run pip install checks"
	@echo "  make lint-yaml       - Lint workflow YAML files (requires yamllint)"

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

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

clippy:
	cargo clippy -- -D warnings

clean:
	cargo clean

example: release
	@model=""; \
	for candidate in $(EXAMPLE_MODEL_CANDIDATES); do \
		if [ -f "$$candidate" ]; then \
			model="$$candidate"; \
			break; \
		fi; \
	done; \
	if [ -z "$$model" ]; then \
		echo "No model found in ~/.loopsleuth/models/ or ./models/"; \
		echo "Run 'loopsleuth download-model' or './setup.sh' to download a model"; \
		exit 1; \
	fi; \
	./target/release/loopsleuth --model "$$model" ./tests/checks/quadratic.py

run: release
	@echo "Usage: make run MODEL=<path> PATH=<python_path>"
	@if [ -z "$(MODEL)" ] || [ -z "$(PATH)" ]; then \
		echo "Example: make run MODEL=./models/model.gguf PATH=./tests/checks/"; \
		exit 1; \
	fi
	./target/release/loopsleuth --model $(MODEL) $(PATH)

golden-update:
	LOOPSLEUTH_TEST_MODEL=$(GOLDEN_TEST_MODEL) \
		python3 tests/run_checks.py --update-golden

golden-verify:
	LOOPSLEUTH_TEST_MODEL=$(GOLDEN_TEST_MODEL) \
		python3 tests/run_checks.py

test-regression:
	./tests/test_regression.sh

test-bootstrap:
	./tests/test_bootstrap.sh

test-pip-install:
	./tests/test_pip_install.sh

lint-yaml:
	yamllint .github/workflows/*.yml
