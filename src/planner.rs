use crate::{Catalog, Deployment, Error, Selection, reason::Reason};
use std::collections::{BTreeMap, BTreeSet};

pub fn plan(catalog: &Catalog, selection: &Selection) -> Result<Deployment, Error> {
    let roots = match selection {
        Selection::Default => &catalog.project.units,
        Selection::Explicit(roots) => roots,
    };
    if roots.is_empty() {
        return Err(Error::EmptySelection);
    }
    let system = &catalog.project.boot.system;
    let mut reasons: BTreeMap<String, BTreeSet<Reason>> = BTreeMap::new();
    reasons
        .entry(system.clone())
        .or_default()
        .insert(Reason::Boot);
    for root in roots {
        if !catalog.units.contains_key(root) {
            return Err(Error::UnknownUnit {
                owner: "selection".into(),
                id: root.clone(),
            });
        }
        reasons
            .entry(root.clone())
            .or_default()
            .insert(Reason::Selected);
    }
    let mut pending: BTreeSet<String> = reasons.keys().cloned().collect();
    let mut visited = BTreeSet::new();
    while let Some(id) = pending.pop_first() {
        if !visited.insert(id.clone()) {
            continue;
        }
        for required in &catalog.units[&id].unit.control.requires {
            reasons
                .entry(required.clone())
                .or_default()
                .insert(Reason::RequiredBy(id.clone()));
            if !visited.contains(required) {
                pending.insert(required.clone());
            }
        }
    }
    let mut remaining: BTreeMap<String, BTreeSet<String>> = reasons
        .keys()
        .filter(|id| *id != system)
        .map(|id| {
            let after = catalog.units[id]
                .unit
                .control
                .after
                .iter()
                .filter(|dependency| *dependency != system && reasons.contains_key(*dependency))
                .cloned()
                .collect();
            (id.clone(), after)
        })
        .collect();
    let mut order = vec![system.clone()];
    while !remaining.is_empty() {
        let Some(next) = remaining
            .iter()
            .find(|(_, after)| after.is_empty())
            .map(|(id, _)| id.clone())
        else {
            return Err(Error::Cycle(remaining.into_keys().collect()));
        };
        remaining.remove(&next);
        for after in remaining.values_mut() {
            after.remove(&next);
        }
        order.push(next);
    }
    Ok(Deployment {
        system: system.clone(),
        order,
        reasons: reasons
            .into_iter()
            .map(|(id, reasons)| (id, reasons.into_iter().collect()))
            .collect(),
    })
}
