//! File-format evolution is independent from application, command and plugin versions.
use crate::*;
use serde_json::Value;

pub const CURRENT_MAJOR: u32 = 1;
pub const CURRENT_MINOR: u32 = 0;
#[derive(Clone, Copy, Debug)]
pub struct FeatureSpec {
    pub name: &'static str,
    pub introduced_minor: u32,
    pub dependencies: &'static [&'static str],
}
pub const FEATURE_REGISTRY: &[FeatureSpec] = &[
    FeatureSpec {
        name: "idef0",
        introduced_minor: 0,
        dependencies: &[],
    },
    FeatureSpec {
        name: "idef0.decomposition",
        introduced_minor: 0,
        dependencies: &["idef0"],
    },
    FeatureSpec {
        name: "idef0.tunnels",
        introduced_minor: 0,
        dependencies: &["idef0"],
    },
    FeatureSpec {
        name: "idef0.calls",
        introduced_minor: 0,
        dependencies: &["idef0"],
    },
    FeatureSpec {
        name: "abc.costs",
        introduced_minor: 0,
        dependencies: &["idef0"],
    },
    FeatureSpec {
        name: "appearance.colors",
        introduced_minor: 0,
        dependencies: &["idef0"],
    },
];
pub struct Migration {
    pub from_minor: u32,
    pub apply: fn(&mut Value) -> Result<()>,
}
// Add one pure migration per released minor step. Never rewrite old migrations.
// v1.0 is the first supported format, so there are no legacy steps yet.
pub const MIGRATIONS: &[Migration] = &[];
pub fn migrate(value: &mut Value) -> Result<()> {
    let major = value["format_version"]["major"]
        .as_u64()
        .ok_or_else(|| error("invalid_document", "Missing major version"))?;
    if value["format"] != "erlu" || major != CURRENT_MAJOR as u64 {
        return Err(error(
            "unsupported_format",
            "Expected erlu format major 1; legacy Qt files are not supported",
        ));
    }
    let mut minor = value["format_version"]["minor"]
        .as_u64()
        .ok_or_else(|| error("invalid_document", "Missing minor version"))?;
    while minor < CURRENT_MINOR as u64 {
        let step = MIGRATIONS
            .iter()
            .find(|m| m.from_minor as u64 == minor)
            .ok_or_else(|| error("migration_missing", format!("No migration from 1.{minor}")))?;
        (step.apply)(value)?;
        minor += 1;
        value["format_version"]["minor"] = minor.into();
    }
    Ok(())
}
pub fn check_features(doc: &Document) -> Result<()> {
    for name in doc
        .features
        .required
        .iter()
        .chain(doc.features.optional.iter())
    {
        if let Some(spec) = FEATURE_REGISTRY.iter().find(|s| s.name == name) {
            if spec.introduced_minor > doc.format_version.minor {
                return Err(error(
                    "invalid_document",
                    format!("Feature {name} requires minor {}", spec.introduced_minor),
                ));
            }
            for dependency in spec.dependencies {
                if !doc.features.required.contains(*dependency)
                    && !doc.features.optional.contains(*dependency)
                {
                    return Err(error(
                        "invalid_document",
                        format!("Feature {name} requires {dependency}"),
                    ));
                }
            }
        } else if doc.features.required.contains(name) {
            return Err(error(
                "unsupported_feature",
                format!("Required feature: {name}"),
            ));
        }
    }
    Ok(())
}
