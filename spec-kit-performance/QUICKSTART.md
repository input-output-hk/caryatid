# Performance Testing with Spec-Kit

This directory contains a spec-kit project for developing comprehensive performance tests for the Caryatid pub/sub framework.

## Quick Start

### 1. Install Prerequisites

```bash
# Install uv package manager
curl -LsSf https://astral.sh/uv/install.sh | sh

# Install spec-kit CLI
uv tool install specify-cli --from git+https://github.com/github/spec-kit.git
```

### 2. Navigate to Spec-Kit Project

```bash
cd spec-kit-performance
```

### 3. Review the Specifications

- **[speckit.constitution](speckit.constitution)**: Project principles and constraints
- **[speckit.specify](speckit.specify)**: Requirements and user stories
- **[speckit.plan](speckit.plan)**: Technical implementation plan
- **[README.md](README.md)**: Complete documentation

### 4. Use AI Agent to Implement

With your AI coding assistant (GitHub Copilot, Claude Code, etc.) active:

```
/speckit.tasks      # Generate actionable task list
/speckit.implement  # Execute implementation
```

## What Gets Created

After implementation, you'll have a new performance test suite at:
```
examples/performance-suite/
```

This suite will provide:
- 📊 Comprehensive metrics (throughput, latency percentiles, resource usage)
- 🎯 Multiple test scenarios (simple, multi-publisher, high-throughput, etc.)
- 🔄 Support for both in-memory and RabbitMQ buses
- 📈 Statistical analysis with HDR histograms
- 📝 Multiple output formats (console, JSON, CSV, markdown)
- ✅ CI/CD integration for regression detection

## Documentation

See [spec-kit-performance/README.md](spec-kit-performance/README.md) for complete documentation on:
- Using spec-kit commands
- Understanding the specification
- Running the implemented tests
- Iterative development

## Why Spec-Kit?

Spec-Kit uses specification-driven development to ensure:
- ✨ Clear requirements before implementation
- 📋 Structured, maintainable code
- 🎯 Alignment with project goals
- 📚 Comprehensive documentation
- 🔄 Easy iteration and enhancement

Learn more at: https://github.com/github/spec-kit
