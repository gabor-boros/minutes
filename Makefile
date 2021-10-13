.PHONY: help prerequisites deps format lint test bench coverage-report build release changelog docs clean
.DEFAULT_GOAL := build

BIN_NAME := minutes

# NOTE: Set in CI/CD as well
COVERAGE_OUT := .coverage.out
COVERAGE_HTML := coverage.html

help: ## Show available targets
	@echo "Available targets:"
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

prerequisites: ## Download and install prerequisites
	go install github.com/goreleaser/goreleaser@latest
	go install github.com/sqs/goreturns@latest
	go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest
	go install github.com/securego/gosec/v2/cmd/gosec@latest

deps: ## Download dependencies
	go mod download
	go mod tidy

format: ## Run formatter on the project
	goreturns -b -local -p -w -e -l .

lint: format ## Run linters on the project
	golangci-lint run --timeout 5m -E golint -e '(struct field|type|method|func) [a-zA-Z`]+ should be [a-zA-Z`]+'
	gosec -quiet ./...

test: deps ## Run tests
	go test -race -cover -covermode=atomic -coverprofile .coverage.out ./...

bench: deps ## Run benchmarks
	# ^$ filters out every unit test, so only benchmarks will run
	go test -run "^$" -benchmem -bench . ./...

coverage-report: ## Generate coverage report from previous test run
	go tool cover -html "$(COVERAGE_OUT)" -o "$(COVERAGE_HTML)"

build: deps ## Build binary
	goreleaser build --rm-dist --snapshot --single-target
	@find bin -name "$(BIN_NAME)" -exec cp "{}" bin/ \;

release: ## Release a new version on GitHub
	goreleaser release --rm-dist

changelog: ## Generate changelog
	git-cliff --unreleased --prepend CHANGELOG.md

docs: ## Serve the documentation site locally
	@cd www && mkdocs serve

clean: ## Clean up project root
	rm -rf bin/ "$(COVERAGE_OUT)" "$(COVERAGE_HTML)"
	go clean -r -i -cache -testcache -modcache
