.PHONY: all build test-simple test-high-throughput test-multi-publisher test-multi-subscriber test-large-messages test-slow-consumer clean

# Variables
PERF_SUITE = ../../target/release/performance_suite
SCENARIOS_DIR = scenarios

# Default target: build and run all scenarios
all: build test-simple test-high-throughput test-multi-publisher test-multi-subscriber test-large-messages test-slow-consumer

# Build the performance suite in release mode
build:
	@echo "Building performance suite..."
	cd ../.. && cargo build --release --package performance_suite

# Run individual test scenarios
test-simple:
	@echo "\n=== Running Simple Scenario ==="
	@$(PERF_SUITE) $(SCENARIOS_DIR)/simple.toml
	@sleep 1

test-high-throughput:
	@echo "\n=== Running High Throughput Scenario ==="
	@$(PERF_SUITE) $(SCENARIOS_DIR)/high_throughput.toml
	@sleep 1

test-multi-publisher:
	@echo "\n=== Running Multi-Publisher Scenario ==="
	@$(PERF_SUITE) $(SCENARIOS_DIR)/multi_publisher.toml
	@sleep 1

test-multi-subscriber:
	@echo "\n=== Running Multi-Subscriber Scenario ==="
	@$(PERF_SUITE) $(SCENARIOS_DIR)/multi_subscriber.toml
	@sleep 1

test-large-messages:
	@echo "\n=== Running Large Messages Scenario ==="
	@$(PERF_SUITE) $(SCENARIOS_DIR)/large_messages.toml
	@sleep 1

test-slow-consumer:
	@echo "\n=== Running Slow Consumer Scenario ==="
	@$(PERF_SUITE) $(SCENARIOS_DIR)/slow_consumer.toml

# Clean build artifacts
clean:
	cd ../.. && cargo clean
	rm -rf ../../results/
