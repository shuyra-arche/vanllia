use crate::{Error, config::Project, image::Image, unit::Declaration};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Catalog {
    pub(crate) project: Project,
    pub(crate) units: BTreeMap<String, Declaration>,
    pub(crate) images: BTreeMap<String, Image>,
}

impl Catalog {
    pub fn project(&self) -> &Project {
        &self.project
    }
    pub fn units(&self) -> &BTreeMap<String, Declaration> {
        &self.units
    }
    pub fn images(&self) -> &BTreeMap<String, Image> {
        &self.images
    }
}

pub fn index(
    project: Project,
    units: Vec<Declaration>,
    images: Vec<Image>,
) -> Result<Catalog, Error> {
    let mut unit_index: BTreeMap<String, Declaration> = BTreeMap::new();
    for declaration in units {
        declaration
            .unit
            .validate()
            .map_err(|why| Error::Declaration {
                path: declaration.path.clone(),
                message: why.to_string(),
            })?;
        let id = declaration.unit.id.clone();
        if let Some(previous) = unit_index.get(&id) {
            return Err(Error::Duplicate {
                kind: "Unit ID",
                id,
                first: previous.path.clone(),
                second: declaration.path,
            });
        }
        unit_index.insert(id, declaration);
    }
    let mut image_index: BTreeMap<String, Image> = BTreeMap::new();
    for image in images {
        if let Some(previous) = image_index.get(&image.name) {
            if previous == &image {
                continue;
            }
            return Err(Error::Duplicate {
                kind: "Cargo bin",
                id: image.name,
                first: previous.manifest.clone(),
                second: image.manifest,
            });
        }
        image_index.insert(image.name.clone(), image);
    }
    for declaration in unit_index.values() {
        let unit = &declaration.unit;
        if !image_index.contains_key(&unit.loader.image) {
            return Err(Error::UnknownImage {
                unit: unit.id.clone(),
                image: unit.loader.image.clone(),
            });
        }
        for id in unit.control.requires.iter().chain(&unit.control.after) {
            if !unit_index.contains_key(id) {
                return Err(Error::UnknownUnit {
                    owner: format!("Unit `{}` ({})", unit.id, declaration.path.display()),
                    id: id.clone(),
                });
            }
        }
    }
    for id in std::iter::once(&project.boot.system).chain(&project.units) {
        if !unit_index.contains_key(id) {
            return Err(Error::UnknownUnit {
                owner: "vanllia.toml".into(),
                id: id.clone(),
            });
        }
    }
    let system = &unit_index[&project.boot.system];
    if !system.unit.control.requires.is_empty() || !system.unit.control.after.is_empty() {
        return Err(Error::Declaration {
            path: system.path.clone(),
            message: "the System boot entry cannot depend on Units it has not loaded yet".into(),
        });
    }
    Ok(Catalog {
        project,
        units: unit_index,
        images: image_index,
    })
}
