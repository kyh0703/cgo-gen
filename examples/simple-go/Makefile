default: build

GO              := go
CURRENT_DIR     := .
BUILD_DIR       := $(CURRENT_DIR)/build
BUILD_BIN       := $(BUILD_DIR)/bin
GO_CACHE        := $(abspath $(CURRENT_DIR)/.gocache)
TARGET          := simple-go
CMD             := $(CURRENT_DIR)/cmd/simple-go

.PHONY: gen
gen:
	@echo "🔄 Generating wrapper files..."
	@cd ../.. && cargo run --bin cgo-gen -- generate --config examples/simple-go/config.yaml --dump-ir
	@echo "✅ Generation complete"

.PHONY: build
build: gen
	@echo "🔨 Building Go application..."
	@mkdir -p $(BUILD_BIN)
	@mkdir -p $(GO_CACHE)
	@GOCACHE=$(GO_CACHE) CGO_ENABLED=1 $(GO) build -o $(BUILD_BIN)/$(TARGET) $(CMD)
	@echo "✅ Build complete: $(BUILD_BIN)/$(TARGET)"

.PHONY: run
run: build
	@echo "🔄 Running Go application..."
	@$(BUILD_BIN)/$(TARGET)

.PHONY: clean
clean:
	@echo "🧹 Cleaning up..."
	@rm -rf $(BUILD_DIR)
	@rm -rf $(GO_CACHE)
	@rm -f $(CURRENT_DIR)/pkg/foo/foo_wrapper.h
	@rm -f $(CURRENT_DIR)/pkg/foo/foo_wrapper.cpp
	@rm -f $(CURRENT_DIR)/pkg/foo/foo_wrapper.ir.yaml
	@echo "✅ Clean complete"
