
# Enforce bash as the shell for consistency
SHELL := bash
# Use bash strict mode
.SHELLFLAGS := -eu -o pipefail -c
MAKEFLAGS += --warn-undefined-variables
MAKEFLAGS += --no-builtin-rules
MAKEFLAGS += --no-print-directory

CRATES_DIR := ./crates
ROOT := $(shell pwd)

# Get list of projects that have makefiles
MAKEFILES=$(wildcard ${CRATES_DIR}/*/*/Makefile)
MAKEFILE_PROJECTS=$(foreach makefile,$(MAKEFILES),$(dir $(makefile)))

# Get list of root crates in $CRATES_DIR
ROOT_RUST_CRATES=$(foreach crate,$(wildcard ${CRATES_DIR}/*/Cargo.toml),$(dir $(crate)))

TEST_WASM_DIR=$(CRATES_DIR)/integration/test-wapc-component
TEST_WASM=$(TEST_WASM_DIR)/build/test_component_s.wasm

TEST_WASI_DIR=$(CRATES_DIR)/integration/test-wasi-component
TEST_WASI=$(TEST_WASI_DIR)/build/test_wasi_component_s.wasm

TEST_PAR=crates/vino/vino-runtime/tests/bundle.tar

VINO_BINS?=vinoc vino vow

RELEASE?=false
ARCH?=local

NATS_URL?=127.0.0.1

ifneq (,$(findstring pc-windows,$(ARCH))) # If arch is *pc-windows*
BIN_SUFFIX:=.exe
else
BIN_SUFFIX:=
endif

##@ Helpers

.PHONY: all
all: build  ## Build everything in this project

# Defines rules for each of the VINO_BINS to copy them into build/local
define BUILD_BIN
$(1): build
	cp target/debug/$$@ build/local
endef

# Call the above rule generator for each BIN file
$(foreach bin,$(VINO_BINS),$(eval $(call BUILD_BIN,$(bin))))

.PHONY: cleangen
cleangen:  ## Run `make clean && make codegen` in child projects
	@for project in $(MAKEFILE_PROJECTS); do \
		echo "# Cleaning $$project"; \
		$(MAKE) -C $$project clean; \
		echo "# Generating code for $$project"; \
		$(MAKE) -C $$project codegen; \
	done

.PHONY: codegen
codegen: ## Run `make codegen` in child projects
	@for project in $(MAKEFILE_PROJECTS); do \
		echo "# Generating code for $$project"; \
		$(MAKE) -C $$project codegen; \
	done

.PHONY: clean
clean:  ## Remove generated artifacts and files
	@rm -rf $(TEST_WASI) $(TEST_WASM)
	@rm -rf ./build/*
	@for project in $(MAKEFILE_PROJECTS); do \
		echo "# Cleaning $$project"; \
		$(MAKE) -C $$project clean; \
	done
	cargo clean

.PHONY: install-release
install-release: $(VINO_BINS)  ## Build optimized vino binaries and install them to your local cargo bin
	cargo build --workspace --release
	cp build/local/* ~/.cargo/bin/

.PHONY: install
install: $(VINO_BINS)  ## Build vino binaries and install them to your local cargo bin
	cargo build --workspace
	cp build/local/* ~/.cargo/bin/

.PHONY: build
build: ./build/local codegen   ## Build the entire vino project
	cargo build --workspace --all

$(TEST_WASM):
	$(MAKE) -C $(TEST_WASM_DIR)

$(TEST_WASI):
	$(MAKE) -C $(TEST_WASI_DIR)

$(TEST_PAR):
	cargo build -p vino-standalone --release
	cargo run -p vinoc -- pack target/release/vino-standalone ./crates/providers/provider-standalone/interface.json -o $@

./build/$(ARCH):
	mkdir -p $@

.PHONY: wasm
wasm: $(TEST_WASM) $(TEST_WASI)   ## Build the test wasm artifacts

.PHONY: test
test: codegen wasm $(TEST_PAR) ## Run tests for the entire workspace
	cargo deny check licenses --hide-inclusion-graph
	cargo build --workspace # necessary to ensure binaries are built
	cargo test --workspace --exclude oci-distribution -- --skip integration_test

.PHONY: test-integration
test-integration: codegen wasm $(TEST_PAR) ## Run all tests for the workspace, including tests that rely on external services
	cargo deny check licenses --hide-inclusion-graph
	cargo build --workspace # necessary to ensure binaries are built
	NATS_URL=$(NATS_URL) cargo test --workspace --exclude oci-distribution

.PHONY: update-lint
update-lint:   ## Update the lint configuration for rust projects
	npm run update-lint

.PHONY: build-tag
build-tag:   ## Tag a build for release
ifeq ($(shell git status -s),)
	@echo Tagging build-$$(date "+%Y-%m-%d")
	@git tag build-$$(date "+%Y-%m-%d") -f
else
	@echo "Check in changes before making a build tag."
endif

.PHONY: bins
bins: ./build/$(ARCH)  ## Build vino bins (supports ARCH & RELEASE env variables)
	@echo "Building ARCH=$(ARCH) RELEASE=$(RELEASE)"
	@rm -rf ./build/$(ARCH)/*
ifeq ($(ARCH),local)
ifeq ($(RELEASE),true)
	cargo build --release $(foreach bin,$(VINO_BINS),-p $(bin))
	cp $(foreach bin,$(VINO_BINS),./target/release/$(bin)$(BIN_SUFFIX)) ./build/$(ARCH)
else
	cargo build $(foreach bin,$(VINO_BINS),-p $(bin))
	cp $(foreach bin,$(VINO_BINS),./target/debug/$(bin)$(BIN_SUFFIX)) ./build/$(ARCH)
endif
else
ifeq ($(RELEASE),true)
ifeq ($(ARCH),x86_64-pc-windows-gnu)
	CARGO_PROFILE_RELEASE_LTO=false cross build --target $(ARCH) --release $(foreach bin,$(VINO_BINS),-p $(bin))
else
	cross build --target $(ARCH) --release $(foreach bin,$(VINO_BINS),-p $(bin))
endif
	cp $(foreach bin,$(VINO_BINS),./target/$(ARCH)/release/$(bin)$(BIN_SUFFIX)) ./build/$(ARCH)
else
	cross build --target $(ARCH) $(foreach bin,$(VINO_BINS),-p $(bin))
	cp $(foreach bin,$(VINO_BINS),./target/$(ARCH)/debug/$(bin)$(BIN_SUFFIX)) ./build/$(ARCH)
endif
endif

.PHONY: deps
deps:   ## Install dependencies
	npm install -g widl-template prettier "https://github.com/vinodotdev/codegen#dev"
	cargo install cargo-deny tomlq

##@ Helpers

.PHONY: list
list: ## Display supported images
	@ls Dockerfile.* | sed -nE 's/Dockerfile\.(.*)/\1/p' | sort

.PHONY: help
help:  ## Display this help
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z0-9_\-.*]+:.*?##/ { printf "  \033[36m%-32s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)
