# Caryatid Performance Testing with Spec-Kit

This directory contains a **Spec-Kit** project for developing comprehensive performance tests for the Caryatid pub/sub framework. The spec-kit approach uses specification-driven development to ensure high-quality, well-documented test infrastructure.

## What is Spec-Kit?

[Spec-Kit](https://github.com/github/spec-kit) is GitHub's toolkit for spec-driven development. Instead of jumping straight to code, you:
1. Define project principles (`speckit.constitution`)
2. Specify requirements (`speckit.specify`)
3. Create an implementation plan (`speckit.plan`)
4. Generate actionable tasks (`speckit.tasks`)
5. Execute implementation (`speckit.implement`)

This approach ensures better quality, maintainability, and alignment with project goals.

## 📁 Project Structure

```
spec-kit-performance/
├── speckit.constitution    # Project principles and constraints
├── speckit.specify          # Performance test requirements & user stories
├── speckit.plan             # Technical implementation plan
├── speckit.tasks            # Generated task list (after running /speckit.tasks)
├── .github/                 # Spec-kit slash commands for AI agents
├── .specify/                # Spec-kit metadata
└── README.md                # This file
```

## 🚀 Getting Started

### Prerequisites

1. **Python 3.11+**: Required by spec-kit
   ```bash
   python3 --version
   ```

2. **uv Package Manager**: For running spec-kit
   ```bash
   # Install uv (macOS/Linux)
   curl -LsSf https://astral.sh/uv/install.sh | sh
   
   # Or using pip
   pip install uv
   ```

3. **Spec-Kit CLI**: Install the specify tool
   ```bash
   uv tool install specify-cli --from git+https://github.com/github/spec-kit.git
   ```

4. **AI Coding Assistant**: One of the following
   - GitHub Copilot (in VS Code/Cursor/etc.)
   - Claude Code
   - Cursor
   - Windsurf
   - Other MCP-compatible assistants

### Verify Installation

```bash
specify check
```

This verifies that all required tools are installed.

## 📝 Spec-Kit Workflow

The spec-kit workflow has already been initialized in this directory. Here's what each file contains:

### 1. Constitution (`speckit.constitution`)
Defines the project's governing principles:
- **Performance First**: Accurate, reproducible metrics
- **Framework Integration**: Seamless integration with Caryatid
- **Observability**: Clear, actionable metrics
- **Flexibility**: Support multiple test scenarios

### 2. Specification (`speckit.specify`)
Describes **what** to build:
- **Requirements**: Comprehensive metrics, multiple scenarios, statistical analysis
- **User Stories**: Developer validation, configuration optimization, capacity planning
- **Success Criteria**: Reproducible results within 5% variance

### 3. Implementation Plan (`speckit.plan`)
Describes **how** to build it:
- **Tech Stack**: Rust + Tokio + HDR Histogram
- **Architecture**: Coordinator, Publishers, Subscribers, Metrics, Reporter
- **Dependencies**: hdrhistogram, sysinfo, prettytable-rs
- **Implementation Phases**: 6 phases from core infrastructure to validation

## 🎯 Using Spec-Kit Commands

With your AI coding assistant active in this directory, you can use the following slash commands:

### Core Workflow

#### 1. Generate Tasks
```
/speckit.tasks
```
This will analyze the plan and generate a detailed, actionable task list in `speckit.tasks`.

#### 2. Implement the Project
```
/speckit.implement
```
This executes all tasks from the task list, creating the actual performance test implementation.

### Enhancement Commands (Optional)

#### Clarify Ambiguities
```
/speckit.clarify
```
Ask structured questions about any ambiguous requirements before implementation.

#### Analyze Consistency
```
/speckit.analyze
```
Cross-check consistency between constitution, specification, plan, and tasks.

#### Generate Quality Checklist
```
/speckit.checklist
```
Create a validation checklist to ensure all requirements are met.

## 🔨 Implementation Output

After running `/speckit.implement`, you should have:

```
caryatid/
├── examples/
│   └── performance-suite/           # NEW: Performance test suite
│       ├── Cargo.toml
│       ├── README.md
│       ├── performance-suite.toml   # Main configuration
│       ├── src/
│       │   ├── main.rs
│       │   ├── config.rs
│       │   ├── coordinator.rs
│       │   ├── publisher_perf.rs
│       │   ├── subscriber_perf.rs
│       │   ├── metrics.rs
│       │   ├── reporter.rs
│       │   └── message.rs
│       ├── scenarios/               # Pre-configured scenarios
│       │   ├── simple.toml
│       │   ├── multi_publisher.toml
│       │   ├── high_throughput.toml
│       │   └── large_messages.toml
│       └── baselines/
│           └── baseline_v1.json
```

## 🏃 Running Performance Tests

Once implemented, run tests like this:

```bash
cd examples/performance-suite

# Build in release mode (critical for accurate results!)
cargo build --release

# Run with default scenario
cargo run --release

# Run specific scenario
cargo run --release -- --scenario scenarios/high_throughput.toml

# Compare with baseline
cargo run --release -- --baseline baselines/baseline_v1.json
```

## 📊 Expected Outputs

### Console Output
```
=== Performance Test Results ===
Scenario: high_throughput
Duration: 60.00s
Messages: 1,000,000
Throughput: 16,667 msg/sec

Latency (μs):
  Min: 12
  p50: 45
  p95: 123
  p99: 287
  Max: 1,543

Resources:
  CPU: 45% avg
  Memory: 128 MB
```

### JSON Output
```json
{
  "scenario": "high_throughput",
  "timestamp": "2024-01-15T10:30:00Z",
  "duration_secs": 60.0,
  "messages": 1000000,
  "throughput": 16667,
  "latency": {
    "min": 12,
    "p50": 45,
    "p90": 98,
    "p95": 123,
    "p99": 287,
    "max": 1543
  }
}
```

## 🎓 Learning Resources

- [Spec-Kit Documentation](https://github.com/github/spec-kit)
- [Spec-Driven Development Methodology](https://github.com/github/spec-kit/blob/main/spec-driven.md)
- [Caryatid Framework](../../README.md)
- [Existing Performance Example](../performance/)

## 🔄 Iterative Development

Spec-Kit supports iterative enhancement:

1. **Update Specification**: Modify `speckit.specify` to add new requirements
2. **Update Plan**: Adjust `speckit.plan` with new technical approach
3. **Regenerate Tasks**: Run `/speckit.tasks` again
4. **Implement Changes**: Run `/speckit.implement` for incremental updates

## 📈 Key Features of This Spec

This performance test specification focuses on:

1. **Comprehensive Metrics**: Throughput, latency (percentiles), resource usage
2. **Multiple Scenarios**: Simple pub/sub, multi-publisher, fan-out, variable message sizes
3. **Bus Configuration Testing**: In-memory vs RabbitMQ
4. **Statistical Analysis**: Proper percentile tracking, reproducibility
5. **Multiple Output Formats**: Console, JSON, CSV, Markdown
6. **CI/CD Integration**: Automated regression detection

## 🤝 Contributing

When extending this performance test suite:

1. Update `speckit.specify` with new requirements
2. Update `speckit.plan` with implementation approach
3. Use `/speckit.tasks` to regenerate task list
4. Use `/speckit.implement` to generate code
5. Test and validate
6. Update this README if needed

## 📝 Notes

- Always run performance tests with `--release` flag
- Results may vary based on hardware and system load
- For CI/CD, consider dedicated performance testing infrastructure
- Baseline measurements should be created on consistent hardware

## 🐛 Troubleshooting

### Spec-Kit Not Working
```bash
# Verify installation
specify check

# Reinstall if needed
uv tool install specify-cli --force --from git+https://github.com/github/spec-kit.git
```

### AI Assistant Not Recognizing Commands
- Ensure you're in the `spec-kit-performance` directory
- Check that `.github/` directory exists with slash command scripts
- Try restarting your AI assistant

### Implementation Issues
Use the clarify command to get help:
```
/speckit.clarify <your question>
```

## 📄 License

This spec-kit project follows the same license as the Caryatid framework (see [LICENSE](../../LICENSE)).
