//! # openConstruct Catalog
//!
//! Tech catalog and module discovery system for openConstruct.
//! Discover, install, and remove tech modules with dependency resolution,
//! platform compatibility checking, and semver version resolution.

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::Path;
use thiserror::Error;

// ── Errors ──────────────────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum CatalogError {
    #[error("module not found: {0}")]
    ModuleNotFound(String),
    #[error("module already registered: {0}")]
    ModuleAlreadyExists(String),
    #[error("circular dependency detected involving: {0}")]
    CircularDependency(String),
    #[error("dependency not found: {0}")]
    DependencyNotFound(String),
    #[error("version conflict for {module}: required {required}, available {available}")]
    VersionConflict {
        module: String,
        required: String,
        available: String,
    },
    #[error("platform incompatible: {details}")]
    PlatformIncompatible { details: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("install failed: {0}")]
    InstallFailed(String),
    #[error("uninstall failed: {0}")]
    UninstallFailed(String),
    #[error("no version of {0} satisfies requirement {1}")]
    NoSatisfyingVersion(String, String),
    #[error("parse error: {0}")]
    ParseError(String),
}

pub type Result<T> = std::result::Result<T, CatalogError>;

// ── Platform ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Browser,
    Embedded,
    Cloud,
    Desktop,
}

impl Platform {
    pub fn all() -> Vec<Platform> {
        vec![
            Platform::Browser,
            Platform::Embedded,
            Platform::Cloud,
            Platform::Desktop,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::Browser => "browser",
            Platform::Embedded => "embedded",
            Platform::Cloud => "cloud",
            Platform::Desktop => "desktop",
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── TechModule ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechModule {
    pub name: String,
    pub version: semver::Version,
    pub description: String,
    pub author: String,
    pub dependencies: Vec<String>,
    pub capabilities: Vec<String>,
    pub repository_url: String,
    #[serde(default)]
    pub supported_platforms: Vec<Platform>,
    #[serde(default)]
    pub min_ram_mb: Option<u32>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl TechModule {
    /// Create a new module builder pattern starter.
    pub fn new(name: impl Into<String>, version: &str) -> Result<Self> {
        Ok(TechModule {
            name: name.into(),
            version: Version::parse(version).map_err(|e| CatalogError::ParseError(e.to_string()))?,
            description: String::new(),
            author: String::new(),
            dependencies: Vec::new(),
            capabilities: Vec::new(),
            repository_url: String::new(),
            supported_platforms: Vec::new(),
            min_ram_mb: None,
            tags: Vec::new(),
        })
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = author.into();
        self
    }

    pub fn dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn capabilities(mut self, caps: Vec<String>) -> Self {
        self.capabilities = caps;
        self
    }

    pub fn repository_url(mut self, url: impl Into<String>) -> Self {
        self.repository_url = url.into();
        self
    }

    pub fn supported_platforms(mut self, platforms: Vec<Platform>) -> Self {
        self.supported_platforms = platforms;
        self
    }

    pub fn min_ram_mb(mut self, mb: u32) -> Self {
        self.min_ram_mb = Some(mb);
        self
    }

    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Unique key combining name and version.
    pub fn key(&self) -> String {
        format!("{}@{}", self.name, self.version)
    }

    /// Check if this module supports a given platform.
    pub fn supports_platform(&self, platform: Platform) -> bool {
        self.supported_platforms.is_empty() || self.supported_platforms.contains(&platform)
    }

    /// Parse dependency string "name" or "name@version-req".
    pub fn parse_dep(dep: &str) -> (String, Option<VersionReq>) {
        if let Some((name, req)) = dep.split_once('@') {
            if let Ok(vr) = VersionReq::parse(req) {
                return (name.to_string(), Some(vr));
            }
        }
        (dep.to_string(), None)
    }
}

// ── CompatibilityReport ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityReport {
    pub module_name: String,
    pub module_version: String,
    pub platform: Platform,
    pub compatible: bool,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
}

impl CompatibilityReport {
    pub fn new(module: &TechModule, platform: Platform) -> Self {
        let mut issues = Vec::new();
        let mut warnings = Vec::new();

        let compatible = module.supports_platform(platform);
        if !compatible {
            issues.push(format!(
                "Module {} does not declare support for {}",
                module.name, platform
            ));
        }

        if let Some(min_ram) = module.min_ram_mb {
            match platform {
                Platform::Embedded => {
                    if min_ram > 64 {
                        warnings.push(format!(
                            "Module requires {}MB RAM — may exceed typical embedded constraints",
                            min_ram
                        ));
                    }
                }
                Platform::Browser => {
                    if min_ram > 512 {
                        warnings.push(format!(
                            "Module requires {}MB RAM — may impact browser performance",
                            min_ram
                        ));
                    }
                }
                _ => {}
            }
        }

        CompatibilityReport {
            module_name: module.name.clone(),
            module_version: module.version.to_string(),
            platform,
            compatible,
            issues,
            warnings,
        }
    }
}

// ── Catalog ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    modules: HashMap<String, Vec<TechModule>>,
}

impl Catalog {
    pub fn new() -> Self {
        Catalog {
            modules: HashMap::new(),
        }
    }

    /// Register a module in the catalog.
    pub fn register(&mut self, module: TechModule) -> Result<()> {
        let versions = self.modules.entry(module.name.clone()).or_default();

        // Check for duplicate exact version
        if versions.iter().any(|m| m.version == module.version) {
            return Err(CatalogError::ModuleAlreadyExists(module.key()));
        }

        versions.push(module);
        Ok(())
    }

    /// Unregister a specific version of a module.
    pub fn unregister(&mut self, name: &str, version: &str) -> Result<()> {
        let versions = self
            .modules
            .get_mut(name)
            .ok_or_else(|| CatalogError::ModuleNotFound(name.to_string()))?;

        let ver = Version::parse(version).map_err(|e| CatalogError::ParseError(e.to_string()))?;
        let before = versions.len();
        versions.retain(|m| m.version != ver);

        if versions.len() == before {
            return Err(CatalogError::ModuleNotFound(format!("{}@{}", name, version)));
        }
        if versions.is_empty() {
            self.modules.remove(name);
        }
        Ok(())
    }

    /// Search modules by query (matches name, description, tags, capabilities).
    pub fn search(&self, query: &str) -> Vec<TechModule> {
        let q = query.to_lowercase();
        let mut results: Vec<TechModule> = self
            .modules
            .values()
            .flat_map(|v| v.iter())
            .filter(|m| {
                m.name.to_lowercase().contains(&q)
                    || m.description.to_lowercase().contains(&q)
                    || m.tags.iter().any(|t| t.to_lowercase().contains(&q))
                    || m.capabilities.iter().any(|c| c.to_lowercase().contains(&q))
            })
            .cloned()
            .collect();

        results.sort_by(|a, b| b.version.cmp(&a.version));
        results
    }

    /// List all modules (latest version of each).
    pub fn list_all(&self) -> Vec<TechModule> {
        let mut result: Vec<TechModule> = self
            .modules
            .values()
            .filter_map(|versions| versions.iter().max_by_key(|m| m.version.clone()))
            .cloned()
            .collect();

        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    }

    /// Get all versions of a module.
    pub fn get_versions(&self, name: &str) -> Vec<&TechModule> {
        self.modules
            .get(name)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Get the latest version of a module.
    pub fn get_latest(&self, name: &str) -> Result<&TechModule> {
        self.modules
            .get(name)
            .and_then(|v| v.iter().max_by_key(|m| m.version.clone()))
            .ok_or_else(|| CatalogError::ModuleNotFound(name.to_string()))
    }

    /// Get a specific version of a module.
    pub fn get(&self, name: &str, version: &str) -> Result<&TechModule> {
        let ver = Version::parse(version).map_err(|e| CatalogError::ParseError(e.to_string()))?;
        self.modules
            .get(name)
            .and_then(|v| v.iter().find(|m| m.version == ver))
            .ok_or_else(|| CatalogError::ModuleNotFound(format!("{}@{}", name, version)))
    }

    /// List all unique module names.
    pub fn module_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.modules.keys().cloned().collect();
        names.sort();
        names
    }

    /// Total number of modules (all versions).
    pub fn total_count(&self) -> usize {
        self.modules.values().map(|v| v.len()).sum()
    }

    /// Parse a catalog from JSON (e.g., from a GitHub org's repo metadata).
    pub fn from_json(json: &str) -> Result<Self> {
        let catalog: Catalog = serde_json::from_str(json)?;
        Ok(catalog)
    }

    /// Serialize the catalog to JSON.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Build a catalog from GitHub-style repo metadata.
    /// Each repo is expected to have name, description, html_url, and optionally
    /// topics (used as tags) and a package.json or Cargo.toml-style version field.
    pub fn from_github_repos(repos: &[serde_json::Value]) -> Result<Self> {
        let mut catalog = Catalog::new();
        for repo in repos {
            let name = repo["name"].as_str().unwrap_or("unknown").to_string();
            let description = repo["description"].as_str().unwrap_or("").to_string();
            let html_url = repo["html_url"].as_str().unwrap_or("").to_string();
            let topics: Vec<String> = repo["topics"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            // Try to extract version from metadata, default to 0.1.0
            let version_str = repo["version"]
                .as_str()
                .or_else(|| repo["default_branch"].as_str().and(Some("0.1.0")))
                .unwrap_or("0.1.0");

            let author = repo["owner"]["login"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();

            let module = TechModule {
                name,
                version: Version::parse(version_str).unwrap_or(Version::new(0, 1, 0)),
                description,
                author,
                dependencies: Vec::new(),
                capabilities: topics.clone(),
                repository_url: html_url,
                supported_platforms: Vec::new(),
                min_ram_mb: None,
                tags: topics,
            };

            catalog.register(module)?;
        }
        Ok(catalog)
    }
}

impl Default for Catalog {
    fn default() -> Self {
        Self::new()
    }
}

// ── DependencyResolver ──────────────────────────────────────────────────────

pub struct DependencyResolver;

impl DependencyResolver {
    /// Resolve all dependencies for a module in topological order.
    /// Returns module names in install order (dependencies first).
    pub fn resolve(module_name: &str, catalog: &Catalog) -> Result<Vec<String>> {
        if Self::detect_cycles(module_name, catalog) {
            return Err(CatalogError::CircularDependency(module_name.to_string()));
        }

        let mut visited = HashSet::new();
        let mut order = Vec::new();
        Self::resolve_dfs(module_name, catalog, &mut visited, &mut order)?;
        Ok(order)
    }

    fn resolve_dfs(
        name: &str,
        catalog: &Catalog,
        visited: &mut HashSet<String>,
        order: &mut Vec<String>,
    ) -> Result<()> {
        if visited.contains(name) {
            return Ok(());
        }

        let module = catalog.get_latest(name)?;

        for dep_str in &module.dependencies {
            let (dep_name, _) = TechModule::parse_dep(dep_str);
            if !visited.contains(&dep_name) {
                Self::resolve_dfs(&dep_name, catalog, visited, order)?;
            }
        }

        visited.insert(name.to_string());
        order.push(name.to_string());
        Ok(())
    }

    /// Detect if a module has circular dependencies.
    pub fn detect_cycles(module_name: &str, catalog: &Catalog) -> bool {
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        Self::dfs_cycle(module_name, catalog, &mut visiting, &mut visited)
    }

    fn dfs_cycle(
        name: &str,
        catalog: &Catalog,
        visiting: &mut HashSet<String>,
        visited: &mut HashSet<String>,
    ) -> bool {
        if visited.contains(name) {
            return false;
        }
        if visiting.contains(name) {
            return true;
        }

        visiting.insert(name.to_string());

        if let Some(modules) = catalog.modules.get(name) {
            if let Some(module) = modules.first() {
                for dep_str in &module.dependencies {
                    let (dep_name, _) = TechModule::parse_dep(dep_str);
                    if catalog.modules.contains_key(&dep_name) && Self::dfs_cycle(&dep_name, catalog, visiting, visited) {
                        return true;
                    }
                }
            }
        }

        visiting.remove(name);
        visited.insert(name.to_string());
        false
    }

    /// Get the full dependency tree for a module (transitive closure).
    pub fn dependency_tree(module_name: &str, catalog: &Catalog) -> Result<HashMap<String, Vec<String>>> {
        let mut tree = HashMap::new();
        let mut queue = VecDeque::new();
        let mut seen = HashSet::new();

        queue.push_back(module_name.to_string());
        seen.insert(module_name.to_string());

        while let Some(name) = queue.pop_front() {
            let module = catalog.get_latest(&name)?;
            let deps: Vec<String> = module
                .dependencies
                .iter()
                .map(|d| TechModule::parse_dep(d).0)
                .collect();

            for dep in &deps {
                if !seen.contains(dep) && catalog.modules.contains_key(dep) {
                    seen.insert(dep.clone());
                    queue.push_back(dep.clone());
                }
            }

            tree.insert(name, deps);
        }

        Ok(tree)
    }
}

// ── CompatibilityChecker ────────────────────────────────────────────────────

pub struct CompatibilityChecker;

impl CompatibilityChecker {
    /// Check compatibility of a module with a platform.
    pub fn check(module: &TechModule, platform: Platform) -> CompatibilityReport {
        CompatibilityReport::new(module, platform)
    }

    /// Check all installed modules against a platform.
    pub fn check_all<'a>(
        modules: impl Iterator<Item = &'a TechModule>,
        platform: Platform,
    ) -> Vec<CompatibilityReport> {
        modules.map(|m| Self::check(m, platform)).collect()
    }
}

// ── VersionResolver ─────────────────────────────────────────────────────────

pub struct VersionResolver;

impl VersionResolver {
    /// Resolve the best matching version of a module given a semver requirement.
    pub fn resolve_best(
        module_name: &str,
        version_req: &str,
        catalog: &Catalog,
    ) -> Result<TechModule> {
        let req = VersionReq::parse(version_req)
            .map_err(|e| CatalogError::ParseError(format!("invalid version req '{}': {}", version_req, e)))?;

        let versions = catalog
            .modules
            .get(module_name)
            .ok_or_else(|| CatalogError::ModuleNotFound(module_name.to_string()))?;

        let mut matching: Vec<&TechModule> = versions.iter().filter(|m| req.matches(&m.version)).collect();

        matching.sort_by(|a, b| b.version.cmp(&a.version));

        matching
            .into_iter()
            .next()
            .cloned()
            .ok_or_else(|| CatalogError::NoSatisfyingVersion(module_name.to_string(), version_req.to_string()))
    }

    /// Get all versions satisfying a requirement.
    pub fn resolve_all(
        module_name: &str,
        version_req: &str,
        catalog: &Catalog,
    ) -> Result<Vec<TechModule>> {
        let req = VersionReq::parse(version_req)
            .map_err(|e| CatalogError::ParseError(format!("invalid version req '{}': {}", version_req, e)))?;

        let versions = catalog
            .modules
            .get(module_name)
            .ok_or_else(|| CatalogError::ModuleNotFound(module_name.to_string()))?;

        let mut matching: Vec<TechModule> = versions
            .iter()
            .filter(|m| req.matches(&m.version))
            .cloned()
            .collect();

        matching.sort_by(|a, b| b.version.cmp(&a.version));
        Ok(matching)
    }
}

// ── ModuleInstaller ─────────────────────────────────────────────────────────

pub struct ModuleInstaller;

impl ModuleInstaller {
    /// Install a module (and its resolved dependencies) to a target directory.
    /// Creates a manifest file per module with metadata.
    pub fn install(
        module_name: &str,
        catalog: &Catalog,
        target_dir: &Path,
    ) -> Result<Vec<String>> {
        let order = DependencyResolver::resolve(module_name, catalog)?;

        fs::create_dir_all(target_dir)?;

        let mut installed = Vec::new();

        for name in &order {
            let module = catalog.get_latest(name)?;
            let module_dir = target_dir.join(&module.name);
            fs::create_dir_all(&module_dir)?;

            // Write manifest
            let manifest = serde_json::to_string_pretty(module)?;
            fs::write(module_dir.join("module.json"), manifest)?;

            // Write a placeholder content file
            fs::write(
                module_dir.join("index.js"),
                format!("// {} v{} — auto-installed by openconstruct-catalog\n", module.name, module.version),
            )?;

            installed.push(module.key());
        }

        // Write lockfile
        let lockfile = serde_json::to_string_pretty(&installed)?;
        fs::write(target_dir.join("catalog.lock"), lockfile)?;

        Ok(installed)
    }

    /// Install with a specific version.
    pub fn install_version(
        module_name: &str,
        version: &str,
        catalog: &Catalog,
        target_dir: &Path,
    ) -> Result<String> {
        let module = catalog.get(module_name, version)?;
        let module_dir = target_dir.join(&module.name);
        fs::create_dir_all(&module_dir)?;

        let manifest = serde_json::to_string_pretty(module)?;
        fs::write(module_dir.join("module.json"), manifest)?;
        fs::write(
            module_dir.join("index.js"),
            format!("// {} v{} — auto-installed by openconstruct-catalog\n", module.name, module.version),
        )?;

        Ok(module.key())
    }

    /// Uninstall a module from the target directory.
    pub fn uninstall(module_name: &str, target_dir: &Path) -> Result<()> {
        let module_dir = target_dir.join(module_name);
        if !module_dir.exists() {
            return Err(CatalogError::UninstallFailed(format!(
                "module {} not found in {}",
                module_name,
                target_dir.display()
            )));
        }
        fs::remove_dir_all(&module_dir)?;
        Ok(())
    }

    /// List installed modules from a target directory.
    pub fn list_installed(target_dir: &Path) -> Result<Vec<String>> {
        if !target_dir.exists() {
            return Ok(Vec::new());
        }

        let mut installed = Vec::new();
        for entry in fs::read_dir(target_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let manifest_path = entry.path().join("module.json");
                if manifest_path.exists() {
                    let content = fs::read_to_string(&manifest_path)?;
                    let module: TechModule = serde_json::from_str(&content)?;
                    installed.push(module.key());
                }
            }
        }

        installed.sort();
        Ok(installed)
    }

    /// Check if a module is installed.
    pub fn is_installed(module_name: &str, target_dir: &Path) -> bool {
        target_dir.join(module_name).join("module.json").exists()
    }
}

// ── Tests (unit) ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_module(name: &str, version: &str) -> TechModule {
        TechModule::new(name, version).unwrap()
            .description(format!("{} module", name))
            .author("test-author")
            .repository_url(format!("https://github.com/test/{}", name))
    }

    #[test]
    fn test_register_and_get() {
        let mut cat = Catalog::new();
        let m = sample_module("foo", "1.0.0");
        cat.register(m).unwrap();
        let got = cat.get("foo", "1.0.0").unwrap();
        assert_eq!(got.name, "foo");
    }

    #[test]
    fn test_duplicate_register_fails() {
        let mut cat = Catalog::new();
        cat.register(sample_module("foo", "1.0.0")).unwrap();
        assert!(cat.register(sample_module("foo", "1.0.0")).is_err());
    }

    #[test]
    fn test_search() {
        let mut cat = Catalog::new();
        cat.register(sample_module("renderer-opengl", "1.0.0").description("OpenGL rendering")).unwrap();
        cat.register(sample_module("physics-bullet", "2.0.0").description("Bullet physics engine")).unwrap();
        cat.register(sample_module("renderer-vulkan", "1.0.0").description("Vulkan rendering")).unwrap();

        let results = cat.search("render");
        assert_eq!(results.len(), 2);
    }
}
