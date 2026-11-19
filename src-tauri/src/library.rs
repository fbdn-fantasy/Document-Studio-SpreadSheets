use crate::{err, files, Document};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

fn new_path(directory: &Path, label: &str) -> PathBuf {
    let clean: String = label.chars().take(72).map(|c| {
        if c.is_control() || "<>:\"/\\|?*".contains(c) { '-' } else { c }
    }).collect();
    let stem = clean.trim_matches(|c| c == ' ' || c == '.');
    let stem = if stem.is_empty() { "Workbook" } else { stem };
    // The prefix also protects against reserved Windows device names.
    let mut name = directory.join(format!("Workbook - {stem}.xlsx"));
    let mut number = 2;
    while name.exists() {
        name = directory.join(format!("Workbook - {stem} ({number}).xlsx"));
        number += 1;
    }
    name
}

pub fn save(directory: &Path, doc: &mut Document) -> Result<(), String> {
    std::fs::create_dir_all(directory).map_err(err)?;
    let previous = doc.path.as_deref().map(Path::new);
    let destination = match previous {
        Some(path) if path.parent() == Some(directory) => path.to_owned(),
        _ => {
            let label = if let Some(stem) = previous.and_then(Path::file_stem) {
                stem.to_string_lossy().into_owned()
            } else if let Some(business) = &doc.meta.business {
                format!("{} {}", business.name, business.period)
            } else {
                doc.meta.settings["custom_template"]["name"].as_str()
                    .filter(|name| !name.trim().is_empty()).unwrap_or(&doc.meta.template).to_owned()
            };
            new_path(directory, &label)
        }
    };
    let path = destination.to_str().ok_or("Unsupported workbook path")?;
    files::save(path, &doc.model, &doc.meta)?;
    doc.path = Some(path.to_owned());
    doc.dirty = false;
    Ok(())
}

pub fn list(directory: &Path) -> Result<Vec<Value>, String> {
    if !directory.exists() { return Ok(Vec::new()); }
    let mut result = Vec::new();
    for entry in std::fs::read_dir(directory).map_err(err)? {
        let entry = entry.map_err(err)?;
        let path = entry.path();
        if !entry.file_type().map_err(err)?.is_file() || !matches!(path.extension().and_then(|s| s.to_str()), Some("xlsx" | "ods")) { continue; }
        let modified = entry.metadata().and_then(|m| m.modified()).map_err(err)?
            .duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
        result.push(json!({"path":path,"name":entry.file_name().to_string_lossy(),"modified":modified}));
    }
    result.sort_by(|a,b| b["modified"].as_u64().cmp(&a["modified"].as_u64()).then_with(|| a["name"].as_str().cmp(&b["name"].as_str())));
    Ok(result)
}
