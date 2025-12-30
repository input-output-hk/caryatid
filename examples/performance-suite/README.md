# Caryatid Performance Test Suite

A comprehensive performance testing suite for the Caryatid pub/sub framework. This suite provides detailed metrics, multiple test scenarios, and statistical analysis to validate performance characteristics and detect regressions.

## Features

- **Comprehensive Metrics**: Throughput, latency percentiles (p50-p99.9), CPU, and memory usage
- **Multiple Scenarios**: Pre-configured tests for different use cases
- **Statistical Analysis**: HDR histogram-based latency tracking with percentile distributions
- **Queue Depth Monitoring**: Track message queuing and back-pressure
- **Multiple Output Formats**: Console, JSON, CSV for different analysis needs
- **Resource Monitoring**: Real-time CPU and memory tracking
- **Configurable**: Easily adjust message sizes, rates, concurrency, and bus types

## Quick Start

### Prerequisites

- Rust 2021 edition or later
- Caryatid framework (this is in the examples/ directory)
- For external bus testing: RabbitMQ (optional)

### Build

```bash
cd examples/performance-suite
cargo build --release
```

**Important**: Always use `--release` mode for accurate performance measurements!

### Run Default Test

```bash
cargo run --release
```

### Run Specific Scenario

```bash
cargo run --release scenarios/simple.toml
cargo run --release scenarios/high_throughput.toml
cargo run --release scenarios/slow_consumer.toml
```

## Test Scenarios

### 1. Simple (`scenarios/simple.toml`)
- **Purpose**: Basic pub/sub validation
- **Config**: 1 publisher, 1 subscriber, 10K messages
- **Duration**: ~30 seconds
- **Use Case**: Quick smoke test, baseline performance

### 2. Multi-Publisher (`scenarios/multi_publisher.toml`)
- **Purpose**: Test concurrent publishing
- **Config**: 4 publishers, 1 subscriber, 100K messages
- **Duration**: ~60 seconds
- **Use Case**: Validate publisher scalability

### 3. Multi-Subscriber (`scenarios/multi_subscriber.toml`)
- **Purpose**: Test message fan-out
- **Config**: 1 publisher, 4 subscribers, 100K messages
- **Duration**: ~60 seconds
- **Use Case**: Validate subscriber scalability

### 4. High Throughput (`scenarios/high_throughput.toml`)
- **Purpose**: Maximum speed test
- **Config**: 4 publishers, 1 subscriber, 1M messages, no rate limit
- **Duration**: ~60 seconds
- **Use Case**: Determine peak throughput

### 5. Large Messages (`scenarios/large_messages.toml`)
- **Purpose**: Test with larger payloads
- **Config**: 2 publishers, 1 subscriber, 50K messages @ 10KB each
- **Duration**: ~60 seconds
- **Use Case**: Validate performance with real-world message sizes

### 6. Slow Consumer (`scenarios/slow_consumer.toml`)
- **Purpose**: Test back-pressure handling
- **Config**: 2 publishers, 1 rate-limited subscriber (500 msg/sec)
- **Duration**: ~60 seconds
- **Use Case**: Validate queue depth tracking and back-pressure

## Configuration

### Basic Configuration Structure

```toml
[coordinator]
output_dir = "./results"
output_formats = ["console", "json", "csv"]

[scenario]
name = "my_test"
description = "Custom test scenario"
warmup_secs = 5
test_duration_secs = 60
cooldown_secs = 2

[module.publisher]
topic = "perf.test"
message_count = 100000
message_size = 100
concurrent_tasks = 1
rate_limit = 1000.0  # Optional: messages/sec

[module.subscriber]
topic = "perf.test"
concurrent_tasks = 1
rate_limit = 500.0  # Optional: for slow consumer

[message-router]
[[message-router.route]]
pattern = "perf.#"
bus = "internal"  # or "external" for RabbitMQ

[message-bus.internal]
bus = "in-memory"
```

### Configuration Parameters

#### Coordinator
- `output_dir`: Directory for results (default: "./results")
- `output_formats`: List of formats: "console", "json", "csv", "markdown"

#### Scenario
- `name`: Scenario identifier
- `description`: Human-readable description
- `warmup_secs`: Warmup period before measurement (default: 5)
- `test_duration_secs`: Main test duration (default: 60)
- `cooldown_secs`: Cooldown after test (default: 2)

#### Publisher
- `topic`: Topic to publish to
- `message_count`: Total messages to send
- `message_size`: Payload size in bytes
- `concurrent_tasks`: Number of parallel publishers
- `rate_limit`: Optional messages/sec limit (omit for unlimited)

#### Subscriber
- `topic`: Topic to subscribe to
- `concurrent_tasks`: Number of parallel subscribers
- `rate_limit`: Optional messages/sec limit for slow consumer testing

## Output Formats

### Console Output

```
======================================================================
Performance Test Results
======================================================================
Scenario: high_throughput
Duration: 60.00s
Messages: 1,000,000
Throughput: 16,667 msg/sec

+-------------+--------+
| Latency (μs)| Value  |
+-------------+--------+
| Min         | 12.50  |
| p50         | 45.20  |
| p90         | 98.30  |
| p95         | 123.40 |
| p99         | 287.60 |
| p99.9       | 654.20 |
| Max         | 1543.80|
| Mean        | 52.30  |
+-------------+--------+

Resources:
  CPU: 45.2% avg
  Memory: 128.5 MB
======================================================================
```

### JSON Output

Results are saved to `{output_dir}/{scenario_name}_results.json`:

```json
{
  "scenario": "high_throughput",
  "timestamp": "2024-01-15T10:30:00Z",
  "duration_secs": 60.0,
  "messages": 1000000,
  "throughput": 16667,
  "latency_micros": {
    "min": 12.5,
    "p50": 45.2,
    "p95": 123.4,
    "p99": 287.6,
    "max": 1543.8
  },
  "resources": {
    "cpu_percent": 45.2,
    "memory_mb": 128.5
  }
}
```

### CSV Output

Results are appended to `{output_dir}/results.csv` for easy spreadsheet analysis.

## Interpreting Results

### Throughput
- **Messages/sec**: Total messages processed per second
- **Good**: > 10,000 msg/sec for small messages
- **Consider**: Message size, concurrent tasks, bus type

### Latency
- **p50 (median)**: Typical latency experienced
- **p95**: 95% of messages faster than this
- **p99**: 99% of messages faster than this
- **p99.9**: Worst 0.1% cases

**Targets** (in-memory bus):
- p50: < 100μs
- p95: < 500μs
- p99: < 1ms

### Queue Depth
- Indicates message buffering
- **Low** (< 10): Good, no back-pressure
- **High** (> 100): Possible back-pressure or slow consumer

### Resources
- **CPU**: Should scale with throughput
- **Memory**: Should remain bounded

## Customizing Tests

### Create Custom Scenario

1. Copy an existing scenario file
2. Modify parameters for your use case
3. Run: `cargo run --release your_scenario.toml`

### Testing External Bus (RabbitMQ)

1. Start RabbitMQ: `docker run -d -p 5672:5672 rabbitmq:3`
2. Update configuration:
   ```toml
   [[message-router.route]]
   pattern = "perf.#"
   bus = "external"
   
   [message-bus.external]
   bus = "rabbit-mq"
   url = "amqp://guest:guest@localhost:5672"
   ```
3. Run test

## Performance Tips

### For Maximum Throughput
- Use `--release` build mode
- Set `concurrent_tasks` based on CPU cores
- Use in-memory bus for lowest latency
- Disable rate limiting
- Use smaller messages (100-1000 bytes)

### For Realistic Testing
- Match production message sizes
- Apply appropriate rate limits
- Test with external bus if used in production
- Run longer tests (5+ minutes) for stability

### For Regression Testing
1. Run baseline: `cargo run --release scenarios/simple.toml`
2. Save baseline results
3. After changes, run same scenario
4. Compare results (throughput, latency percentiles)

## CI/CD Integration

### Example GitHub Actions

```yaml
name: Performance Tests

on: [push, pull_request]

jobs:
  performance:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run performance tests
        run: |
          cd examples/performance-suite
          cargo run --release scenarios/simple.toml
      - name: Upload results
        uses: actions/upload-artifact@v2
        with:
          name: performance-results
          path: examples/performance-suite/results/
```

## Troubleshooting

### Low Throughput
- Ensure using `--release` mode
- Check CPU usage (should be high for throughput tests)
- Verify no rate limits applied
- Check for system resource constraints

### High Latency
- Normal for external bus (network overhead)
- Check for CPU saturation
- Verify no rate limiting on subscriber
- Check queue depth for back-pressure

### Inconsistent Results
- Run longer tests (increase `test_duration_secs`)
- Increase warmup period
- Close other applications
- Run multiple times and average results

### Out of Memory
- Reduce `message_count`
- Reduce `message_size`
- Reduce `concurrent_tasks`
- Check for message queue buildup

## Comparison with Simple Performance Example

| Feature | Simple Example | Performance Suite |
|---------|---------------|-------------------|
| Latency Measurement | ❌ | ✅ Microsecond precision |
| Percentiles | ❌ | ✅ p50-p99.9 |
| Queue Depth | ❌ | ✅ Full tracking |
| Resource Monitoring | ❌ | ✅ CPU & Memory |
| Multiple Scenarios | ⚠️ Manual | ✅ Pre-configured |
| JSON Output | ❌ | ✅ |
| CSV Export | ❌ | ✅ |
| Statistical Analysis | ⚠️ Basic | ✅ HDR Histogram |

## Architecture

### Components

- **Coordinator**: Orchestrates test lifecycle (warmup, test, cooldown)
- **PublisherPerf**: Configurable message publisher with rate limiting
- **SubscriberPerf**: Latency-measuring subscriber with queue depth tracking
- **MetricsCollector**: HDR histogram-based metrics collection
- **Reporter**: Multi-format output generation

### Message Flow

```
Coordinator -> StartTest
    ↓
PublisherPerf -> PerfData messages -> SubscriberPerf
    ↓                                      ↓
Coordinator <- StopTest           Metrics Collection
    ↓                                      ↓
Coordinator <- Metrics messages <- SubscriberPerf
    ↓
Report Generation
```

## Contributing

To add new scenarios:
1. Create TOML file in `scenarios/`
2. Document in this README
3. Test with `cargo run --release your_scenario.toml`

To add new metrics:
1. Update `PerformanceMetrics` in `message.rs`
2. Update `MetricsCollector` in `metrics.rs`
3. Update reporters in `reporter.rs`

## License

Same as Caryatid framework - see root LICENSE file.

## Support

For issues or questions:
- Check existing examples in `scenarios/`
- Review configuration documentation above
- See main Caryatid README for framework details
