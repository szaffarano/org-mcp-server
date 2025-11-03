# justfile for org-mcp-server

# Variables
coverage_dir := "coverage"

# Default recipe - show all available commands
_default:
    @just --list

# === Build Recipes ===

# Build all crates
build:
    cargo build

# Build release version
build-release:
    cargo build --release

# === Test & Coverage Recipes ===

# Run all tests
test:
    cargo test

# Clean build artifacts and coverage reports
clean:
    cargo clean
    rm -rf {{coverage_dir}}/

# Generate HTML coverage report
coverage-html:
    @echo "Generating HTML coverage report..."
    cargo llvm-cov --html --output-dir {{coverage_dir}}/html
    @echo "Coverage report generated in {{coverage_dir}}/html/"
    @echo "Open {{coverage_dir}}/html/index.html in your browser"

# Show coverage summary in terminal
coverage-summary:
    @echo "Running coverage analysis..."
    cargo llvm-cov --summary-only

# Generate coverage for CI (LCOV format)
coverage-ci:
    @echo "Generating coverage report for CI..."
    mkdir -p {{coverage_dir}}
    cargo llvm-cov --lcov --output-path {{coverage_dir}}/lcov.info

# Generate JSON coverage report
coverage-json:
    @echo "Generating JSON coverage report..."
    mkdir -p {{coverage_dir}}
    cargo llvm-cov --json --output-path {{coverage_dir}}/coverage.json

# Generate all coverage formats
coverage: coverage-html coverage-ci coverage-json coverage-summary
    @echo "All coverage reports generated:"
    @echo "  - HTML: {{coverage_dir}}/html/index.html"
    @echo "  - LCOV: {{coverage_dir}}/lcov.info"
    @echo "  - JSON: {{coverage_dir}}/coverage.json"

# === Code Quality Recipes ===

# Run clippy linter
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Check code formatting without modifying
fmt-check:
    cargo fmt --all -- --check

# === Workflow Recipes ===

# Run all quality checks (format check, lint, test, coverage)
check: fmt-check lint test coverage-summary

# Development workflow - format, lint, test, coverage
dev: fmt lint test coverage-summary
