use crate::finance::{validate, Metadata};
use calamine::{open_workbook_auto, Data, Reader};
use icu_locale_core::locale;
use ironcalc::base::UserModel;
use spreadsheet_ods::{Sheet, WorkBook};
use std::{
    fs,
    io::{Cursor, Read, Write},
    path::Path,
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

pub fn load(path: &str) -> Result<(UserModel<'static>, Metadata), String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    if bytes.len() > 100 * 1024 * 1024 {
        return Err("Workbook exceeds the 100 MB safety limit".into());
    }
    let mut archive = ZipArchive::new(Cursor::new(&bytes)).map_err(|e| e.to_string())?;
    let mut expanded = 0u64;
    for i in 0..archive.len() {
        expanded = expanded.saturating_add(archive.by_index(i).map_err(|e| e.to_string())?.size());
        if expanded > 512 * 1024 * 1024 {
            return Err("Expanded workbook exceeds the 512 MB safety limit".into());
        }
    }
    let mut meta = Metadata::default();
    if let Ok(mut part) = archive.by_name("document-studio/metadata.json") {
        let mut text = String::new();
        part.read_to_string(&mut text).map_err(|e| e.to_string())?;
        meta =
            serde_json::from_str(&text).map_err(|e| format!("Invalid business metadata: {e}"))?;
        validate(&meta)?;
    }
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let mut model = match ext.as_str() {
        "xlsx" => UserModel::from_model(
            ironcalc::import::load_from_xlsx(path, "en", "UTC", "en").map_err(|e| e.to_string())?,
        ),
        "ods" => {
            let mut book = open_workbook_auto(path).map_err(|e| e.to_string())?;
            let mut m = UserModel::new_empty("Workbook", "en", "UTC", "en")?;
            m.pause_evaluation();
            for (index, name) in book.sheet_names().to_vec().iter().enumerate() {
                if index > 0 {
                    m.new_sheet()?;
                }
                m.rename_sheet(index as u32, name)?;
                let cells = book.worksheet_range(name).map_err(|e| e.to_string())?;
                for (r, c, value) in cells.used_cells() {
                    let (sr, sc) = cells.start().unwrap_or((0, 0));
                    let content = match value {
                        Data::Empty => continue,
                        Data::String(s) => format!("'{s}"),
                        _ => value.to_string(),
                    };
                    m.set_user_input(
                        index as u32,
                        (r as u32 + sr + 1) as i32,
                        (c as u32 + sc + 1) as i32,
                        &content,
                    )?;
                }
                if let Ok(formulas) = book.worksheet_formula(name) {
                    let (sr, sc) = formulas.start().unwrap_or((0, 0));
                    for (r, c, f) in formulas.used_cells() {
                        if !f.is_empty() {
                            m.set_user_input(
                                index as u32,
                                (r as u32 + sr + 1) as i32,
                                (c as u32 + sc + 1) as i32,
                                &from_ods(f)?,
                            )?;
                        }
                    }
                }
            }
            m.resume_evaluation();
            m
        }
        _ => return Err("Only XLSX and ODS workbooks are supported".into()),
    };
    crate::live::sync_records(&mut meta)?;
    crate::templates::sync(&mut model, &meta)?;
    model.evaluate();
    Ok((model, meta))
}

// Token-aware conversion: separators are changed only outside quoted strings.
pub fn from_ods(input: &str) -> Result<String, String> {
    let source = input
        .strip_prefix("of:=")
        .or_else(|| input.strip_prefix("oooc:="))
        .or_else(|| input.strip_prefix('='))
        .unwrap_or(input);
    let mut result = String::from("=");
    let mut chars = source.chars().peekable();
    let mut quoted = false;
    while let Some(c) = chars.next() {
        if c == '"' {
            quoted = !quoted;
            result.push(c);
            continue;
        }
        if !quoted && c == '[' {
            let mut reference = String::new();
            let mut closed = false;
            for next in chars.by_ref() {
                if next == ']' {
                    closed = true;
                    break;
                }
                reference.push(next);
            }
            if !closed {
                return Err("Invalid OpenFormula reference".into());
            }
            if reference.contains('#') {
                return Err("External workbook formulas are not supported".into());
            }
            let converted = reference
                .split(':')
                .map(|p| {
                    let p = p.strip_prefix('$').unwrap_or(p);
                    if let Some(s) = p.strip_prefix('.') {
                        s.to_string()
                    } else if let Some((s, cell)) = p.rsplit_once('.') {
                        format!("{s}!{cell}")
                    } else {
                        p.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(":");
            result.push_str(&converted);
        } else if !quoted && c == ';' {
            result.push(',');
        } else {
            result.push(c);
        }
    }
    Ok(result)
}
pub fn to_ods(input: &str) -> String {
    let input = input.strip_prefix('=').unwrap_or(input);
    let reference=regex::Regex::new(r"^(?:(?:'[^']*(?:''[^']*)*'|[A-Za-z_][A-Za-z0-9_]*)!)?\$?[A-Za-z]{1,3}\$?[1-9][0-9]*(?::(?:(?:'[^']*(?:''[^']*)*'|[A-Za-z_][A-Za-z0-9_]*)!)?\$?[A-Za-z]{1,3}\$?[1-9][0-9]*)?").unwrap();
    let mut output = String::from("of:=");
    let mut i = 0;
    let mut string = false;
    while i < input.len() {
        let c = input[i..].chars().next().unwrap();
        if c == '"' {
            string = !string;
            output.push(c);
            i += 1;
            continue;
        }
        if !string && (i == 0 || !input[..i].chars().last().unwrap().is_alphanumeric()) {
            if let Some(m) = reference.find(&input[i..]) {
                let end = i + m.end();
                if input[end..].chars().next() != Some('(') {
                    let parts = m
                        .as_str()
                        .split(':')
                        .map(|r| {
                            if let Some((s, c)) = r.rsplit_once('!') {
                                format!("{s}.{c}")
                            } else {
                                format!(".{r}")
                            }
                        })
                        .collect::<Vec<_>>();
                    output.push_str(&format!("[{}]", parts.join(":")));
                    i = end;
                    continue;
                }
            }
        }
        output.push(if !string && c == ',' { ';' } else { c });
        i += c.len_utf8();
    }
    output
}
pub fn save(path: &str, model: &UserModel, meta: &Metadata) -> Result<(), String> {
    validate(meta)?;
    let ext = Path::new(path)
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or("")
        .to_lowercase();
    let mut raw = Cursor::new(Vec::new());
    if ext == "xlsx" {
        raw = ironcalc::export::save_xlsx_to_writer(model.get_model(), raw)
            .map_err(|e| e.to_string())?;
    } else if ext == "ods" {
        let mut book = WorkBook::new(locale!("en-US"));
        for (index, ws) in model.get_model().workbook.worksheets.iter().enumerate() {
            let mut sheet = Sheet::new(&ws.name);
            for (r, c, _) in ws.sheet_data.cells() {
                let text = model.get_cell_content(index as u32, r, c)?;
                let rr = (r - 1) as u32;
                let cc = (c - 1) as u32;
                let displayed = model.get_formatted_cell_value(index as u32, r, c)?;
                if text.starts_with('=') {
                    sheet.set_formula(rr, cc, to_ods(&text));
                    if let Ok(n) = displayed.parse::<f64>() {
                        sheet.set_value(rr, cc, n);
                    } else {
                        sheet.set_value(rr, cc, displayed);
                    }
                } else if let Ok(n) = text.parse::<f64>() {
                    sheet.set_value(rr, cc, n);
                } else {
                    sheet.set_value(rr, cc, text.strip_prefix('\'').unwrap_or(&text));
                }
            }
            book.push_sheet(sheet);
        }
        let tmp = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
        spreadsheet_ods::write_ods(&mut book, tmp.path()).map_err(|e| e.to_string())?;
        raw = Cursor::new(fs::read(tmp.path()).map_err(|e| e.to_string())?);
    } else {
        return Err("Save with an .xlsx or .ods extension".into());
    }
    raw.set_position(0);
    let mut old = ZipArchive::new(raw).map_err(|e| e.to_string())?;
    let mut output = ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..old.len() {
        let mut entry = old.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        if name == "document-studio/metadata.json" {
            continue;
        }
        let mut data = Vec::new();
        entry.read_to_end(&mut data).map_err(|e| e.to_string())?;
        if name == "[Content_Types].xml" {
            data=String::from_utf8_lossy(&data).replace("</Types>","<Override PartName=\"/document-studio/metadata.json\" ContentType=\"application/json\"/></Types>").into_bytes();
        }
        if name == "META-INF/manifest.xml" {
            data=String::from_utf8_lossy(&data).replace("</manifest:manifest>","<manifest:file-entry manifest:full-path=\"document-studio/metadata.json\" manifest:media-type=\"application/json\"/></manifest:manifest>").into_bytes();
        }
        output
            .start_file(
                name,
                SimpleFileOptions::default().compression_method(entry.compression()),
            )
            .map_err(|e| e.to_string())?;
        output.write_all(&data).map_err(|e| e.to_string())?;
    }
    output
        .start_file(
            "document-studio/metadata.json",
            SimpleFileOptions::default(),
        )
        .map_err(|e| e.to_string())?;
    output
        .write_all(&serde_json::to_vec(meta).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let data = output.finish().map_err(|e| e.to_string())?.into_inner();
    let parent = Path::new(path).parent().ok_or("Invalid destination")?;
    let mut staged = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    staged.write_all(&data).map_err(|e| e.to_string())?;
    staged.as_file().sync_all().map_err(|e| e.to_string())?;
    if Path::new(path).exists() {
        fs::copy(path, format!("{path}.backup")).map_err(|e| format!("Backup failed: {e}"))?;
    }
    staged.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formulas() {
        assert_eq!(
            from_ods("of:=SUM([.A1:.B5];\"a;b\")").unwrap(),
            "=SUM(A1:B5,\"a;b\")"
        );
        assert_eq!(to_ods("=SUM(A1:B5,\"a,b\")"), "of:=SUM([.A1:.B5];\"a,b\")");
        assert_eq!(
            from_ods(&to_ods("='My Sheet'!$A$1+SUM(B2:B4)")).unwrap(),
            "='My Sheet'!$A$1+SUM(B2:B4)"
        );
    }
    #[test]
    fn roundtrips() {
        let mut m = UserModel::new_empty("Test", "en", "UTC", "en").unwrap();
        m.set_user_input(0, 1, 1, "4").unwrap();
        m.set_user_input(0, 2, 1, "=A1*3").unwrap();
        let meta = Metadata::default();
        let d = tempfile::tempdir().unwrap();
        for ext in ["xlsx", "ods"] {
            let path = d.path().join(format!("test.{ext}"));
            save(path.to_str().unwrap(), &m, &meta).unwrap();
            let (loaded, mm) = load(path.to_str().unwrap()).unwrap();
            assert_eq!(loaded.get_formatted_cell_value(0, 2, 1).unwrap(), "12");
            assert_eq!(mm.workbook_id, meta.workbook_id);
            save(path.to_str().unwrap(), &loaded, &mm).unwrap();
        }
    }
}
