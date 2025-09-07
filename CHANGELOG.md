# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2024-12-19

### Added
- **Warm-up feature**: Containers are now pre-started on function creation for faster cold starts
- **Enhanced dependency handling**: Smart dependency resolution with vendored dependencies support
- **Improved prime calculator**: Better algorithm for complex computation emulation
- **Enhanced test suite**: Comprehensive e2e tests with proper cleanup using delete API
- **Performance optimizations**: Better container lifecycle management and idle pool handling

### Changed
- **Breaking**: Updated API structure and improved error handling
- **Breaking**: Enhanced configuration system with warm-up settings
- **Breaking**: Improved container management and lifecycle policies

### Fixed
- Fixed dependency installation logic to prioritize vendored dependencies
- Fixed prime calculator algorithm for large number calculations
- Fixed test cleanup to use proper API deletion instead of manual container killing
- Fixed performance issues in sustained load scenarios

## [0.1.0] - 2024-09-05

### Added
- Docker-backed AWS Lambda clone
- Embedded web console
- Multi-runtime support (Node.js, Python, Rust)
- Warm container pool with auto-scaling
- AWS Lambda-compatible APIs
- Prometheus metrics and structured logging
