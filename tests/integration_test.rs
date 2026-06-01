//! Integration tests for openconstruct-catalog

use openconstruct_catalog::*;
use tempfile::TempDir;
use std::path::Path;

// ── Helpers ─────────────────────────────────────────────────────────────────

fn make_module(name: &str, version: &str) -> TechModule {
    TechModule::new(name, version).unwrap()
        .description(format!("{} module", name))
        .author("test-author")
        .repository_url(format!("https://github.com/test/{}", name))
}

fn make_module_with_deps(name: &str, version: &str, deps: Vec<String>) -> TechModule {
    make_module(name, version).dependencies(deps)
}

fn make_catalog() -> Catalog {
    let mut cat = Catalog::new();
    cat.register(make_module("core", "1.0.0")).unwrap();
    cat.register(make_module("math", "1.0.0").dependencies(vec!["core".into()])).unwrap();
    cat.register(make_module("renderer", "2.0.0").dependencies(vec!["core".into(), "math".into()])).unwrap();
    cat.register(make_module("physics", "1.5.0").dependencies(vec!["core".into(), "math".into()])).unwrap();
    cat
}

// ── 1. Catalog Registration Tests ───────────────────────────────────────────

#[test]
fn test_register_basic() {
    let mut cat = Catalog::new();
    let m = make_module("foo", "1.0.0");
    cat.register(m).unwrap();
    assert_eq!(cat.module_names(), vec!["foo"]);
}

#[test]
fn test_register_multiple_versions() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    cat.register(make_module("foo", "1.1.0")).unwrap();
    cat.register(make_module("foo", "2.0.0")).unwrap();
    assert_eq!(cat.get_versions("foo").len(), 3);
}

#[test]
fn test_register_duplicate_fails() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    let err = cat.register(make_module("foo", "1.0.0"));
    assert!(err.is_err());
    match err {
        Err(CatalogError::ModuleAlreadyExists(name)) => assert!(name.contains("foo")),
        _ => panic!("expected ModuleAlreadyExists"),
    }
}

#[test]
fn test_register_different_modules() {
    let mut cat = Catalog::new();
    cat.register(make_module("alpha", "1.0.0")).unwrap();
    cat.register(make_module("beta", "1.0.0")).unwrap();
    cat.register(make_module("gamma", "1.0.0")).unwrap();
    assert_eq!(cat.module_names().len(), 3);
}

#[test]
fn test_unregister_module() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    cat.unregister("foo", "1.0.0").unwrap();
    assert!(cat.get("foo", "1.0.0").is_err());
}

#[test]
fn test_unregister_one_version_keeps_others() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    cat.register(make_module("foo", "2.0.0")).unwrap();
    cat.unregister("foo", "1.0.0").unwrap();
    assert!(cat.get("foo", "1.0.0").is_err());
    assert!(cat.get("foo", "2.0.0").is_ok());
}

#[test]
fn test_unregister_nonexistent_fails() {
    let mut cat = Catalog::new();
    assert!(cat.unregister("nope", "1.0.0").is_err());
}

// ── 2. Search Tests ─────────────────────────────────────────────────────────

#[test]
fn test_search_by_name() {
    let cat = make_catalog();
    let results = cat.search("renderer");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "renderer");
}

#[test]
fn test_search_by_description() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0").description("A physics simulation engine")).unwrap();
    cat.register(make_module("bar", "1.0.0").description("A JSON parser")).unwrap();
    let results = cat.search("physics");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "foo");
}

#[test]
fn test_search_by_tag() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0").tags(vec!["graphics".into(), "3d".into()])).unwrap();
    cat.register(make_module("bar", "1.0.0").tags(vec!["audio".into()])).unwrap();
    let results = cat.search("graphics");
    assert_eq!(results.len(), 1);
}

#[test]
fn test_search_by_capability() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0").capabilities(vec!["webgl".into()])).unwrap();
    let results = cat.search("webgl");
    assert_eq!(results.len(), 1);
}

#[test]
fn test_search_case_insensitive() {
    let mut cat = Catalog::new();
    cat.register(make_module("FooBar", "1.0.0")).unwrap();
    let results = cat.search("foobar");
    assert_eq!(results.len(), 1);
}

#[test]
fn test_search_empty_query() {
    let cat = make_catalog();
    let results = cat.search("");
    assert_eq!(results.len(), 4);
}

#[test]
fn test_search_no_results() {
    let cat = make_catalog();
    let results = cat.search("nonexistent");
    assert!(results.is_empty());
}

#[test]
fn test_search_returns_latest_first() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    cat.register(make_module("foo", "2.0.0")).unwrap();
    cat.register(make_module("foo", "1.5.0")).unwrap();
    let results = cat.search("foo");
    assert_eq!(results[0].version.major, 2);
}

// ── 3. List All Tests ───────────────────────────────────────────────────────

#[test]
fn test_list_all_returns_latest_versions() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    cat.register(make_module("foo", "2.0.0")).unwrap();
    cat.register(make_module("bar", "1.0.0")).unwrap();
    let all = cat.list_all();
    assert_eq!(all.len(), 2);
    let foo = all.iter().find(|m| m.name == "foo").unwrap();
    assert_eq!(foo.version.major, 2);
}

#[test]
fn test_list_all_sorted_by_name() {
    let mut cat = Catalog::new();
    cat.register(make_module("zeta", "1.0.0")).unwrap();
    cat.register(make_module("alpha", "1.0.0")).unwrap();
    cat.register(make_module("mid", "1.0.0")).unwrap();
    let all = cat.list_all();
    assert_eq!(all[0].name, "alpha");
    assert_eq!(all[1].name, "mid");
    assert_eq!(all[2].name, "zeta");
}

#[test]
fn test_list_all_empty() {
    let cat = Catalog::new();
    assert!(cat.list_all().is_empty());
}

#[test]
fn test_total_count() {
    let mut cat = Catalog::new();
    cat.register(make_module("a", "1.0.0")).unwrap();
    cat.register(make_module("a", "2.0.0")).unwrap();
    cat.register(make_module("b", "1.0.0")).unwrap();
    assert_eq!(cat.total_count(), 3);
}

// ── 4. Dependency Resolution Tests ──────────────────────────────────────────

#[test]
fn test_resolve_simple() {
    let mut cat = Catalog::new();
    cat.register(make_module("a", "1.0.0").dependencies(vec!["b".into()])).unwrap();
    cat.register(make_module("b", "1.0.0")).unwrap();
    let order = DependencyResolver::resolve("a", &cat).unwrap();
    assert_eq!(order, vec!["b", "a"]);
}

#[test]
fn test_resolve_deep_chain() {
    let mut cat = Catalog::new();
    cat.register(make_module("a", "1.0.0").dependencies(vec!["b".into()])).unwrap();
    cat.register(make_module("b", "1.0.0").dependencies(vec!["c".into()])).unwrap();
    cat.register(make_module("c", "1.0.0")).unwrap();
    let order = DependencyResolver::resolve("a", &cat).unwrap();
    assert_eq!(order, vec!["c", "b", "a"]);
}

#[test]
fn test_resolve_diamond_dependency() {
    // a depends on b and c, both b and c depend on d
    let mut cat = Catalog::new();
    cat.register(make_module("a", "1.0.0").dependencies(vec!["b".into(), "c".into()])).unwrap();
    cat.register(make_module("b", "1.0.0").dependencies(vec!["d".into()])).unwrap();
    cat.register(make_module("c", "1.0.0").dependencies(vec!["d".into()])).unwrap();
    cat.register(make_module("d", "1.0.0")).unwrap();
    let order = DependencyResolver::resolve("a", &cat).unwrap();

    // d must come before b and c, b and c must come before a
    let pos = |name: &str| order.iter().position(|n| n == name).unwrap();
    assert!(pos("d") < pos("b"));
    assert!(pos("d") < pos("c"));
    assert!(pos("b") < pos("a"));
    assert!(pos("c") < pos("a"));
    assert_eq!(order.len(), 4);
}

#[test]
fn test_resolve_no_deps() {
    let mut cat = Catalog::new();
    cat.register(make_module("standalone", "1.0.0")).unwrap();
    let order = DependencyResolver::resolve("standalone", &cat).unwrap();
    assert_eq!(order, vec!["standalone"]);
}

#[test]
fn test_resolve_missing_dependency() {
    let mut cat = Catalog::new();
    cat.register(make_module("a", "1.0.0").dependencies(vec!["missing".into()])).unwrap();
    let result = DependencyResolver::resolve("a", &cat);
    assert!(result.is_err());
}

#[test]
fn test_resolve_full_catalog() {
    let cat = make_catalog();
    let order = DependencyResolver::resolve("renderer", &cat).unwrap();
    // renderer -> core, math; math -> core
    let pos = |name: &str| order.iter().position(|n| n == name).unwrap();
    assert!(pos("core") < pos("math"));
    assert!(pos("core") < pos("renderer"));
    assert!(pos("math") < pos("renderer"));
}

// ── 5. Cycle Detection Tests ───────────────────────────────────────────────

#[test]
fn test_detect_no_cycle() {
    let mut cat = Catalog::new();
    cat.register(make_module("a", "1.0.0").dependencies(vec!["b".into()])).unwrap();
    cat.register(make_module("b", "1.0.0")).unwrap();
    assert!(!DependencyResolver::detect_cycles("a", &cat));
}

#[test]
fn test_detect_simple_cycle() {
    let mut cat = Catalog::new();
    cat.register(make_module("a", "1.0.0").dependencies(vec!["b".into()])).unwrap();
    cat.register(make_module("b", "1.0.0").dependencies(vec!["a".into()])).unwrap();
    assert!(DependencyResolver::detect_cycles("a", &cat));
}

#[test]
fn test_detect_three_node_cycle() {
    let mut cat = Catalog::new();
    cat.register(make_module("a", "1.0.0").dependencies(vec!["b".into()])).unwrap();
    cat.register(make_module("b", "1.0.0").dependencies(vec!["c".into()])).unwrap();
    cat.register(make_module("c", "1.0.0").dependencies(vec!["a".into()])).unwrap();
    assert!(DependencyResolver::detect_cycles("a", &cat));
}

#[test]
fn test_detect_self_cycle() {
    let mut cat = Catalog::new();
    cat.register(make_module("a", "1.0.0").dependencies(vec!["a".into()])).unwrap();
    assert!(DependencyResolver::detect_cycles("a", &cat));
}

#[test]
fn test_cycle_prevents_resolve() {
    let mut cat = Catalog::new();
    cat.register(make_module("a", "1.0.0").dependencies(vec!["b".into()])).unwrap();
    cat.register(make_module("b", "1.0.0").dependencies(vec!["a".into()])).unwrap();
    let result = DependencyResolver::resolve("a", &cat);
    assert!(matches!(result, Err(CatalogError::CircularDependency(_))));
}

#[test]
fn test_no_cycle_with_diamond() {
    let mut cat = Catalog::new();
    cat.register(make_module("a", "1.0.0").dependencies(vec!["b".into(), "c".into()])).unwrap();
    cat.register(make_module("b", "1.0.0").dependencies(vec!["d".into()])).unwrap();
    cat.register(make_module("c", "1.0.0").dependencies(vec!["d".into()])).unwrap();
    cat.register(make_module("d", "1.0.0")).unwrap();
    assert!(!DependencyResolver::detect_cycles("a", &cat));
}

// ── 6. Platform Compatibility Tests ─────────────────────────────────────────

#[test]
fn test_compatible_platform() {
    let m = make_module("foo", "1.0.0")
        .supported_platforms(vec![Platform::Browser, Platform::Desktop]);
    let report = CompatibilityChecker::check(&m, Platform::Browser);
    assert!(report.compatible);
    assert!(report.issues.is_empty());
}

#[test]
fn test_incompatible_platform() {
    let m = make_module("foo", "1.0.0")
        .supported_platforms(vec![Platform::Desktop]);
    let report = CompatibilityChecker::check(&m, Platform::Embedded);
    assert!(!report.compatible);
    assert!(!report.issues.is_empty());
}

#[test]
fn test_no_platform_restriction_is_compatible() {
    let m = make_module("foo", "1.0.0"); // no platforms specified
    for plat in Platform::all() {
        let report = CompatibilityChecker::check(&m, plat);
        assert!(report.compatible, "should be compatible with {}", plat);
    }
}

#[test]
fn test_embedded_ram_warning() {
    let m = make_module("foo", "1.0.0")
        .supported_platforms(vec![Platform::Embedded])
        .min_ram_mb(128);
    let report = CompatibilityChecker::check(&m, Platform::Embedded);
    assert!(report.compatible);
    assert!(!report.warnings.is_empty());
}

#[test]
fn test_browser_ram_warning() {
    let m = make_module("foo", "1.0.0")
        .supported_platforms(vec![Platform::Browser])
        .min_ram_mb(1024);
    let report = CompatibilityChecker::check(&m, Platform::Browser);
    assert!(report.compatible);
    assert!(!report.warnings.is_empty());
}

#[test]
fn test_check_all_platforms() {
    let modules = vec![
        make_module("a", "1.0.0").supported_platforms(vec![Platform::Desktop]),
        make_module("b", "1.0.0").supported_platforms(vec![Platform::Cloud]),
    ];
    let reports = CompatibilityChecker::check_all(modules.iter(), Platform::Desktop);
    assert_eq!(reports.len(), 2);
    assert!(reports[0].compatible);
    assert!(!reports[1].compatible);
}

// ── 7. Version Resolution Tests ─────────────────────────────────────────────

#[test]
fn test_resolve_exact_version() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    cat.register(make_module("foo", "2.0.0")).unwrap();
    let m = VersionResolver::resolve_best("foo", "=1.0.0", &cat).unwrap();
    assert_eq!(m.version.to_string(), "1.0.0");
}

#[test]
fn test_resolve_caret_range() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    cat.register(make_module("foo", "1.1.0")).unwrap();
    cat.register(make_module("foo", "2.0.0")).unwrap();
    let m = VersionResolver::resolve_best("foo", "^1", &cat).unwrap();
    assert_eq!(m.version.to_string(), "1.1.0");
}

#[test]
fn test_resolve_tilde_range() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    cat.register(make_module("foo", "1.0.1")).unwrap();
    cat.register(make_module("foo", "1.1.0")).unwrap();
    let m = VersionResolver::resolve_best("foo", "~1.0.0", &cat).unwrap();
    assert_eq!(m.version.to_string(), "1.0.1");
}

#[test]
fn test_resolve_greater_than() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    cat.register(make_module("foo", "1.5.0")).unwrap();
    cat.register(make_module("foo", "2.0.0")).unwrap();
    let m = VersionResolver::resolve_best("foo", ">1.0.0, <2.0.0", &cat).unwrap();
    assert_eq!(m.version.to_string(), "1.5.0");
}

#[test]
fn test_resolve_wildcard() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    cat.register(make_module("foo", "1.2.0")).unwrap();
    cat.register(make_module("foo", "2.0.0")).unwrap();
    let m = VersionResolver::resolve_best("foo", "1.*", &cat).unwrap();
    assert_eq!(m.version.to_string(), "1.2.0");
}

#[test]
fn test_resolve_no_matching_version() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    let result = VersionResolver::resolve_best("foo", ">=2.0.0", &cat);
    assert!(result.is_err());
}

#[test]
fn test_resolve_missing_module() {
    let cat = Catalog::new();
    let result = VersionResolver::resolve_best("nope", "^1", &cat);
    assert!(result.is_err());
}

#[test]
fn test_resolve_all_matching() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    cat.register(make_module("foo", "1.1.0")).unwrap();
    cat.register(make_module("foo", "1.2.0")).unwrap();
    cat.register(make_module("foo", "2.0.0")).unwrap();
    let all = VersionResolver::resolve_all("foo", "^1", &cat).unwrap();
    assert_eq!(all.len(), 3);
    // sorted highest first
    assert_eq!(all[0].version.to_string(), "1.2.0");
}

#[test]
fn test_resolve_best_picks_latest() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    cat.register(make_module("foo", "1.9.0")).unwrap();
    cat.register(make_module("foo", "1.5.0")).unwrap();
    let m = VersionResolver::resolve_best("foo", "^1", &cat).unwrap();
    assert_eq!(m.version.to_string(), "1.9.0");
}

// ── 8. Install/Uninstall Lifecycle Tests ────────────────────────────────────

#[test]
fn test_install_simple() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    let dir = TempDir::new().unwrap();
    let installed = ModuleInstaller::install("foo", &cat, dir.path()).unwrap();
    assert_eq!(installed, vec!["foo@1.0.0"]);
    assert!(dir.path().join("foo/module.json").exists());
    assert!(dir.path().join("foo/index.js").exists());
}

#[test]
fn test_install_with_dependencies() {
    let cat = make_catalog();
    let dir = TempDir::new().unwrap();
    let installed = ModuleInstaller::install("renderer", &cat, dir.path()).unwrap();
    assert!(installed.contains(&"core@1.0.0".to_string()));
    assert!(installed.contains(&"math@1.0.0".to_string()));
    assert!(installed.contains(&"renderer@2.0.0".to_string()));
    assert_eq!(installed.len(), 3);
}

#[test]
fn test_install_creates_directories() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    let dir = TempDir::new().unwrap();
    ModuleInstaller::install("foo", &cat, dir.path()).unwrap();
    assert!(dir.path().join("foo").is_dir());
}

#[test]
fn test_install_creates_lockfile() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    let dir = TempDir::new().unwrap();
    ModuleInstaller::install("foo", &cat, dir.path()).unwrap();
    assert!(dir.path().join("catalog.lock").exists());
}

#[test]
fn test_uninstall() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    let dir = TempDir::new().unwrap();
    ModuleInstaller::install("foo", &cat, dir.path()).unwrap();
    ModuleInstaller::uninstall("foo", dir.path()).unwrap();
    assert!(!dir.path().join("foo").exists());
}

#[test]
fn test_uninstall_nonexistent_fails() {
    let dir = TempDir::new().unwrap();
    let result = ModuleInstaller::uninstall("nope", dir.path());
    assert!(result.is_err());
}

#[test]
fn test_list_installed() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    cat.register(make_module("bar", "2.0.0")).unwrap();
    let dir = TempDir::new().unwrap();
    ModuleInstaller::install("foo", &cat, dir.path()).unwrap();
    ModuleInstaller::install("bar", &cat, dir.path()).unwrap();
    let installed = ModuleInstaller::list_installed(dir.path()).unwrap();
    assert_eq!(installed.len(), 2);
    assert!(installed.contains(&"bar@2.0.0".to_string()));
    assert!(installed.contains(&"foo@1.0.0".to_string()));
}

#[test]
fn test_list_installed_empty() {
    let dir = TempDir::new().unwrap();
    let installed = ModuleInstaller::list_installed(dir.path()).unwrap();
    assert!(installed.is_empty());
}

#[test]
fn test_list_installed_nonexistent_dir() {
    let installed = ModuleInstaller::list_installed(Path::new("/tmp/no-such-dir-xyz")).unwrap();
    assert!(installed.is_empty());
}

#[test]
fn test_is_installed() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    let dir = TempDir::new().unwrap();
    assert!(!ModuleInstaller::is_installed("foo", dir.path()));
    ModuleInstaller::install("foo", &cat, dir.path()).unwrap();
    assert!(ModuleInstaller::is_installed("foo", dir.path()));
}

#[test]
fn test_install_specific_version() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    cat.register(make_module("foo", "2.0.0")).unwrap();
    let dir = TempDir::new().unwrap();
    let key = ModuleInstaller::install_version("foo", "1.0.0", &cat, dir.path()).unwrap();
    assert_eq!(key, "foo@1.0.0");
}

// ── 9. Serialization Tests ──────────────────────────────────────────────────

#[test]
fn test_catalog_to_json() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    let json = cat.to_json().unwrap();
    assert!(json.contains("foo"));
    assert!(json.contains("1.0.0"));
}

#[test]
fn test_catalog_from_json() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0").description("test")).unwrap();
    let json = cat.to_json().unwrap();
    let parsed = Catalog::from_json(&json).unwrap();
    let m = parsed.get_latest("foo").unwrap();
    assert_eq!(m.description, "test");
}

#[test]
fn test_module_json_roundtrip() {
    let m = make_module("foo", "1.2.3")
        .tags(vec!["graphics".into()])
        .capabilities(vec!["render".into()])
        .supported_platforms(vec![Platform::Browser, Platform::Desktop]);
    let json = serde_json::to_string_pretty(&m).unwrap();
    let parsed: TechModule = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, m.name);
    assert_eq!(parsed.tags, m.tags);
    assert_eq!(parsed.supported_platforms, m.supported_platforms);
}

// ── 10. GitHub Repo Catalog Tests ──────────────────────────────────────────

#[test]
fn test_from_github_repos() {
    let repos = vec![
        serde_json::json!({
            "name": "oc-renderer",
            "description": "A rendering engine",
            "html_url": "https://github.com/openconstruct/oc-renderer",
            "topics": ["graphics", "rendering"],
            "owner": { "login": "openconstruct" },
            "version": "1.2.0"
        }),
        serde_json::json!({
            "name": "oc-physics",
            "description": "Physics simulation",
            "html_url": "https://github.com/openconstruct/oc-physics",
            "topics": ["physics", "simulation"],
            "owner": { "login": "openconstruct" }
        }),
    ];

    let cat = Catalog::from_github_repos(&repos).unwrap();
    assert_eq!(cat.module_names().len(), 2);
    let renderer = cat.get_latest("oc-renderer").unwrap();
    assert_eq!(renderer.version.to_string(), "1.2.0");
    assert!(renderer.tags.contains(&"graphics".to_string()));
    let physics = cat.get_latest("oc-physics").unwrap();
    assert_eq!(physics.version.to_string(), "0.1.0"); // default
}

#[test]
fn test_from_github_repos_empty() {
    let cat = Catalog::from_github_repos(&[]).unwrap();
    assert!(cat.list_all().is_empty());
}

#[test]
fn test_from_github_repos_minimal_fields() {
    let repos = vec![
        serde_json::json!({
            "name": "minimal",
        }),
    ];
    let cat = Catalog::from_github_repos(&repos).unwrap();
    let m = cat.get_latest("minimal").unwrap();
    assert_eq!(m.name, "minimal");
    assert_eq!(m.author, "unknown");
}

// ── 11. Dependency Tree Tests ───────────────────────────────────────────────

#[test]
fn test_dependency_tree_simple() {
    let mut cat = Catalog::new();
    cat.register(make_module("a", "1.0.0").dependencies(vec!["b".into()])).unwrap();
    cat.register(make_module("b", "1.0.0")).unwrap();
    let tree = DependencyResolver::dependency_tree("a", &cat).unwrap();
    assert_eq!(tree.get("a").unwrap().len(), 1);
    assert!(tree.get("a").unwrap().contains(&"b".to_string()));
}

#[test]
fn test_dependency_tree_complex() {
    let cat = make_catalog();
    let tree = DependencyResolver::dependency_tree("renderer", &cat).unwrap();
    assert!(tree.contains_key("renderer"));
    assert!(tree.contains_key("core"));
    assert!(tree.contains_key("math"));
}

// ── 12. Platform Enum Tests ─────────────────────────────────────────────────

#[test]
fn test_platform_all() {
    let all = Platform::all();
    assert_eq!(all.len(), 4);
    assert!(all.contains(&Platform::Browser));
    assert!(all.contains(&Platform::Embedded));
    assert!(all.contains(&Platform::Cloud));
    assert!(all.contains(&Platform::Desktop));
}

#[test]
fn test_platform_display() {
    assert_eq!(Platform::Browser.to_string(), "browser");
    assert_eq!(Platform::Embedded.to_string(), "embedded");
    assert_eq!(Platform::Cloud.to_string(), "cloud");
    assert_eq!(Platform::Desktop.to_string(), "desktop");
}

// ── 13. TechModule Builder Tests ────────────────────────────────────────────

#[test]
fn test_module_key() {
    let m = make_module("foo", "1.2.3");
    assert_eq!(m.key(), "foo@1.2.3");
}

#[test]
fn test_module_new_invalid_version() {
    let result = TechModule::new("foo", "not-a-version");
    assert!(result.is_err());
}

#[test]
fn test_module_supports_platform() {
    let m = make_module("foo", "1.0.0")
        .supported_platforms(vec![Platform::Cloud]);
    assert!(m.supports_platform(Platform::Cloud));
    assert!(!m.supports_platform(Platform::Browser));
}

#[test]
fn test_parse_dep_simple() {
    let (name, req) = TechModule::parse_dep("core");
    assert_eq!(name, "core");
    assert!(req.is_none());
}

#[test]
fn test_parse_dep_with_version() {
    let (name, req) = TechModule::parse_dep("core@^1.0");
    assert_eq!(name, "core");
    assert!(req.is_some());
}

// ── 14. Edge Case Tests ────────────────────────────────────────────────────

#[test]
fn test_catalog_default() {
    let cat = Catalog::default();
    assert!(cat.list_all().is_empty());
}

#[test]
fn test_get_latest_picks_highest() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    cat.register(make_module("foo", "3.0.0")).unwrap();
    cat.register(make_module("foo", "2.0.0")).unwrap();
    let latest = cat.get_latest("foo").unwrap();
    assert_eq!(latest.version.major, 3);
}

#[test]
fn test_get_specific_version() {
    let mut cat = Catalog::new();
    cat.register(make_module("foo", "1.0.0")).unwrap();
    cat.register(make_module("foo", "2.0.0")).unwrap();
    let m = cat.get("foo", "1.0.0").unwrap();
    assert_eq!(m.version.major, 1);
}

#[test]
fn test_compatibility_report_serialize() {
    let m = make_module("foo", "1.0.0").supported_platforms(vec![Platform::Desktop]);
    let report = CompatibilityReport::new(&m, Platform::Browser);
    let json = serde_json::to_string(&report).unwrap();
    assert!(json.contains("browser"));
}

#[test]
fn test_install_diamond_dependency() {
    let mut cat = Catalog::new();
    cat.register(make_module("a", "1.0.0").dependencies(vec!["b".into(), "c".into()])).unwrap();
    cat.register(make_module("b", "1.0.0").dependencies(vec!["d".into()])).unwrap();
    cat.register(make_module("c", "1.0.0").dependencies(vec!["d".into()])).unwrap();
    cat.register(make_module("d", "1.0.0")).unwrap();
    let dir = TempDir::new().unwrap();
    let installed = ModuleInstaller::install("a", &cat, dir.path()).unwrap();
    // d should appear exactly once
    let d_count = installed.iter().filter(|k| k.starts_with("d@")).count();
    assert_eq!(d_count, 1);
    assert_eq!(installed.len(), 4);
}

