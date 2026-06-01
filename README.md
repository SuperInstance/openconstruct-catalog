# openconstruct-catalog

> Module discovery, dependency resolution, and semantic versioning for plugin ecosystems

## What This Does

openconstruct-catalog is a Rust crate that provides a catalog system for discovering, registering, and resolving dependencies between modules in a plugin ecosystem. It handles semantic versioning constraints, platform compatibility checks, and topological dependency ordering — the plumbing that lets a modular system know what to load and in what order.

## The Key Idea

Think of it as `cargo` for plugins. You declare modules with version requirements and platform constraints. The catalog builds a dependency graph, checks for conflicts, does topological sorting, and tells you exactly which modules to load and in what order. No circular dependencies. No version conflicts. No loading a Linux module on Windows.

## Install

```toml
[dependencies]
openconstruct-catalog = { git = "https://github.com/SuperInstance/openconstruct-catalog" }
```

## Quick Start

```rust
use openconstruct_catalog::*;

// Register modules
let mut catalog = Catalog::new();

catalog.register(ModuleEntry {
    name: "core-math".into(),
    version: Version::parse("1.2.0").unwrap(),
    dependencies: vec![],
    platforms: vec![Platform::Linux, Platform::MacOS],
});

catalog.register(ModuleEntry {
    name: "spectral-ops".into(),
    version: Version::parse("2.0.0").unwrap(),
    dependencies: vec![Dependency {
        name: "core-math".into(),
        version_req: VersionReq::parse(">=1.0.0").unwrap(),
    }],
    platforms: vec![Platform::Linux],
});

// Resolve: returns modules in load order
let resolved = catalog.resolve().unwrap();
assert_eq!(resolved[0].name, "core-math"); // dependency first
assert_eq!(resolved[1].name, "spectral-ops");
```

## API Reference

### `Version`

Semantic version (major.minor.patch).

| Method | Description |
|--------|-------------|
| `Version::parse("1.2.3")` | Parse a semver string. Returns `Result<Version>`. |
| `major()`, `minor()`, `patch()` | Access version components. |
| `satisfies(req: &VersionReq)` | Check if this version meets a requirement. |

### `VersionReq`

Semver requirement (e.g., `>=1.0.0`, `^2.0`, `~1.2.3`).

| Method | Description |
|--------|-------------|
| `VersionReq::parse(">=1.0.0")` | Parse a version requirement string. |
| `matches(v: &Version)` | Check if a version satisfies this requirement. |

### `ModuleEntry`

A module registration entry.

| Field | Description |
|-------|-------------|
| `name: String` | Unique module identifier. |
| `version: Version` | Module version. |
| `dependencies: Vec<Dependency>` | Required dependencies. |
| `platforms: Vec<Platform>` | Supported platforms. |

### `Dependency`

| Field | Description |
|-------|-------------|
| `name: String` | Required module name. |
| `version_req: VersionReq` | Version constraint. |

### `Platform`

Enum: `Linux`, `MacOS`, `Windows`, `Wasm`.

### `Catalog`

| Method | Description |
|--------|-------------|
| `Catalog::new()` | Create an empty catalog. |
| `register(entry: ModuleEntry)` | Register a module. |
| `resolve()` | Resolve dependencies, check compatibility, return topologically sorted modules. |
| `list()` | List all registered modules. |
| `find(name: &str)` | Look up a module by name. |

### Errors

| Error | When |
|-------|------|
| `CircularDependency` | Dependency graph has a cycle. |
| `VersionConflict` | No version satisfies all constraints. |
| `PlatformIncompatible` | Module doesn't support current platform. |
| `MissingDependency` | Required module not registered. |

## How It Works

1. **Registration**: Modules are added with their metadata, dependencies, and platform support.
2. **Dependency Graph**: The catalog builds a directed graph from dependency declarations.
3. **Cycle Detection**: DFS-based cycle detection rejects circular dependencies.
4. **Version Resolution**: For each dependency, the catalog checks that registered versions satisfy the version requirements.
5. **Platform Filtering**: Modules incompatible with the current platform are excluded.
6. **Topological Sort**: Kahn's algorithm produces a load order where every module appears after its dependencies.

## Testing

80 tests covering:
- Semver parsing and comparison (major, minor, patch, pre-release)
- Version requirement matching (`>=`, `^`, `~`, `*`)
- Dependency resolution with multiple modules
- Circular dependency detection
- Platform compatibility filtering
- Topological sort correctness
- Edge cases: empty catalogs, self-dependencies, diamond dependencies

## License

MIT
