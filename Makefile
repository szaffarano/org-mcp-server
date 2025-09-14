# Makefile for org-mcp-server

.PHONY: build test clean coverage coverage-html coverage-summary coverage-ci lint fmt

# Build all crates
build:
	cargo build

# Build release
build-release:
	cargo build --release

# Run all tests
test:
	cargo test

# Clean build artifacts
clean:
	cargo clean
	rm -rf coverage/

# Run tests with coverage and generate HTML report
coverage-html:
	@echo "Generating HTML coverage report..."
	cargo llvm-cov --html --output-dir coverage/html
	@echo "Coverage report generated in coverage/html/"
	@echo "Open coverage/html/index.html in your browser"

# Show coverage summary in terminal
coverage-summary:
	@echo "Running coverage analysis..."
	cargo llvm-cov --summary-only

# Generate coverage for CI (LCOV format)
coverage-ci:
	@echo "Generating coverage report for CI..."
	mkdir -p coverage
	cargo llvm-cov --lcov --output-path coverage/lcov.info

# Generate JSON coverage report
coverage-json:
	@echo "Generating JSON coverage report..."
	mkdir -p coverage
	cargo llvm-cov --json --output-path coverage/coverage.json

# Generate all coverage formats
coverage: coverage-html coverage-ci coverage-json coverage-summary
	@echo "All coverage reports generated:"
	@echo "  - HTML: coverage/html/index.html"
	@echo "  - LCOV: coverage/lcov.info"
	@echo "  - JSON: coverage/coverage.json"

# Run clippy linter
lint:
	cargo clippy --all-targets --all-features -- -D warnings

# Format code
fmt:
	cargo fmt --all

# Check formatting
fmt-check:
	cargo fmt --all -- --check

# Run all quality checks
check: fmt-check lint test coverage-summary

# Development workflow - format, lint, test, coverage
dev: fmt lint test coverage-summary

# Show help
help:
	@echo "Available targets:"
	@echo "  build          - Build all crates"
	@echo "  build-release  - Build release version"
	@echo "  test           - Run all tests"
	@echo "  clean          - Clean build artifacts and coverage reports"
	@echo "  coverage       - Generate all coverage reports"
	@echo "  coverage-html  - Generate HTML coverage report"
	@echo "  coverage-summary - Show coverage summary in terminal"
	@echo "  coverage-ci    - Generate LCOV format for CI"
	@echo "  coverage-json  - Generate JSON coverage report"
	@echo "  lint           - Run clippy linter"
	@echo "  fmt            - Format code"
	@echo "  fmt-check      - Check code formatting"
	@echo "  check          - Run all quality checks"
	@echo "  dev            - Development workflow (format, lint, test, coverage)"
	@echo "  help           - Show this help message"