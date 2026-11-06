use std::path::Path;

use crate::ImageBuild;

/// Explicit declaration/build facts exported by one prepared source root.
pub struct Provider<'a> {
    pub root: &'a Path,
    pub declarations: &'a [&'static unit::UnitFile],
    pub builds: &'a [ImageBuild],
}

/// Gather provider facts without imposing a product catalog or repository layout.
/// Manifest coordinates become absolute, anchored at each actual source root.
pub fn collect(
    providers: &[Provider<'_>],
    target: &str,
) -> Result<(Vec<&'static unit::UnitFile>, Vec<ImageBuild>), String> {
    let mut declarations: Vec<&'static unit::UnitFile> = Vec::new();
    let mut builds = Vec::new();
    for provider in providers {
        let root = provider.root.canonicalize().map_err(|e| e.to_string())?;
        for unit in provider.declarations {
            if declarations
                .iter()
                .any(|previous| previous.name() == unit.name())
            {
                return Err(format!("conflicting Unit {}", unit.name()));
            }
            declarations.push(*unit);
        }
        for build in provider.builds {
            if build.target != target {
                return Err(format!(
                    "image {} target {} incompatible with {target}",
                    build.image, build.target
                ));
            }
            let manifest = root
                .join(build.manifest)
                .canonicalize()
                .map_err(|e| format!("{} manifest: {e}", build.image))?;
            if !manifest.starts_with(&root) || !manifest.is_file() {
                return Err(format!("{} manifest escapes provider root", build.image));
            }
            let mut build = *build;
            build.manifest = Box::leak(manifest.to_string_lossy().into_owned().into_boxed_str());
            builds.push(build);
        }
    }
    crate::validate_builds(&builds)?;
    Ok((declarations, builds))
}
