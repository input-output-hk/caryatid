# Performance Testing with Spec-Kit

A comprehensive, specification-driven performance testing suite for Caryatid is being developed using GitHub's Spec-Kit methodology.

## 📍 Location

The spec-kit project is located at:
```
../spec-kit-performance/
```

## 🎯 What is This?

This is a **Spec-Kit project** that uses specification-driven development to create a production-ready performance testing infrastructure. Instead of jumping straight to code, we:

1. ✅ Define project principles (`speckit.constitution`)
2. ✅ Specify requirements (`speckit.specify`)  
3. ✅ Create implementation plan (`speckit.plan`)
4. ⏭️ Generate actionable tasks (`/speckit.tasks`)
5. ⏭️ Execute implementation (`/speckit.implement`)

## 🚀 Getting Started

### Quick Start

```bash
# 1. Install spec-kit
uv tool install specify-cli --from git+https://github.com/github/spec-kit.git

# 2. Navigate to the project
cd ../spec-kit-performance

# 3. Read the documentation
cat README.md
```

### With AI Assistant

In the `spec-kit-performance/` directory, use:
```
/speckit.tasks      # Generate task list
/speckit.implement  # Create the test suite
```

## 📦 What Gets Created

After running `/speckit.implement`, you'll have:

```
examples/
├── performance/              ← Existing simple test
└── performance-suite/        ← NEW comprehensive test suite
    ├── src/
    │   ├── coordinator.rs    # Test orchestration
    │   ├── publisher_perf.rs # Advanced publisher
    │   ├── subscriber_perf.rs # Latency measurement
    │   ├── metrics.rs        # HDR histogram stats
    │   └── reporter.rs       # JSON/CSV/Console output
    ├── scenarios/            # Pre-configured tests
    └── baselines/            # Regression detection
```

## 🎁 Features

Compared to the existing `performance/` example, the spec-kit suite adds:

- 📊 **Latency Analysis**: Microsecond precision, p50/p95/p99 percentiles
- 📈 **Resource Monitoring**: CPU and memory tracking
- 🎯 **Multiple Scenarios**: Pre-configured test scenarios
- 📝 **Multiple Outputs**: Console, JSON, CSV, Markdown
- ✅ **CI/CD Integration**: Baseline comparison, exit codes
- 🔄 **Test Orchestration**: Automated lifecycle management
- 📊 **Statistical Analysis**: HDR histograms, confidence intervals

## 📚 Documentation

See the spec-kit project for complete documentation:
- [README.md](../spec-kit-performance/README.md) - Complete guide
- [QUICKSTART.md](../spec-kit-performance/QUICKSTART.md) - Quick reference
- [COMPARISON.md](../spec-kit-performance/COMPARISON.md) - vs existing test
- [SETUP_SUMMARY.md](../spec-kit-performance/SETUP_SUMMARY.md) - What's been done

## 🤔 When to Use What?

### Use `performance/` (existing) when:
- Quick sanity check needed
- Testing basic message passing  
- Simple throughput measurement

### Use `performance-suite/` (spec-kit) when:
- Need detailed latency analysis
- Comparing configurations
- Performance regression testing
- Capacity planning
- Publishing benchmarks
- Production validation

## 🎓 Learn More

- **Spec-Kit**: https://github.com/github/spec-kit
- **Specification**: [../spec-kit-performance/speckit.specify](../spec-kit-performance/speckit.specify)
- **Implementation Plan**: [../spec-kit-performance/speckit.plan](../spec-kit-performance/speckit.plan)

## 🔗 Links

- [Existing Performance Test](./performance/)
- [Spec-Kit Project](../spec-kit-performance/)
- [Caryatid Framework](../README.md)
