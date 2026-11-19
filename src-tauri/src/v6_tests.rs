use super::*;
use crate::finance::{Business, Record, Vehicle};

fn blank() -> Document {
    let (model, meta) = templates::create("Blank workbook", None).unwrap();
    Document {model, meta, path: None, dirty: true}
}

#[test]
fn library_creates_distinct_files_and_reopens_the_latest_saved_edit() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("Workbooks");
    let mut first = blank();
    library::save(&dir, &mut first).unwrap();
    let path = first.path.clone().unwrap();
    let mut second = blank();
    library::save(&dir, &mut second).unwrap();
    assert_ne!(first.path, second.path);
    first.model.set_user_input(0, 1, 1, "Edited later").unwrap();
    first.dirty = true;
    library::save(&dir, &mut first).unwrap();
    assert!(!first.dirty);
    assert_eq!(first.path.as_deref(), Some(path.as_str()));
    assert_eq!(library::list(&dir).unwrap().len(), 2);
    assert_eq!(files::load(&path).unwrap().0.get_cell_content(0, 1, 1).unwrap(), "Edited later");
}

#[test]
fn library_saves_imports_as_copies_and_retains_failed_edits() {
    let temp = tempfile::tempdir().unwrap();
    let mut doc = blank();
    let original = temp.path().join("Old file.ods");
    let original = original.to_str().unwrap().to_string();
    files::save(&original, &doc.model, &doc.meta).unwrap();
    let before = std::fs::read(&original).unwrap();
    doc.path = Some(original.clone());
    doc.model.set_user_input(0, 1, 1, "Newest value").unwrap();
    assert!(library::save(std::path::Path::new(&original), &mut doc).is_err());
    assert!(doc.dirty);
    assert_eq!(doc.path.as_deref(), Some(original.as_str()));
    library::save(&temp.path().join("Workbooks"), &mut doc).unwrap();
    assert_ne!(doc.path.as_deref(), Some(original.as_str()));
    assert_eq!(std::fs::read(original).unwrap(), before);
}

#[test]
fn custom_business_tables_follow_the_sheet_after_save_and_reopen() {
    let business = Business {company_id:"v6".into(),name:"Fleet".into(),period:"2026-11".into(),country:"South Africa".into(),currency:"ZAR".into(),year_start:1,..Default::default()};
    let (mut model, mut meta) = templates::create("Custom Template",Some(business)).unwrap();
    meta.settings["custom_template"] = json!({"sheets":{
        "Fuel":{"enabled":true,"columns":{"Tax percent":false,"Tax fixed":false,"Tax included":false,"Deductible percent":false,"Deductible fixed":false,"Deductible base":false,"Deduction treatment":false,"Deductible amount":false}},
        "Income":{"enabled":true,"columns":{"Invoice number":false,"Tax":false}}
    }});
    meta.records.push(Record {id:"tank".into(),revision:1,category:"Fuel".into(),period:"2026-11".into(),party:"Fuel stop".into(),amount:"600".into(),vehicle:Vehicle {litres:"30".into(),price_per_litre:"20".into(),start_odometer:"1000".into(),end_odometer:"1300".into(),..Default::default()},..Default::default()});
    live::sync_records(&mut meta).unwrap();
    templates::sync(&mut model,&meta).unwrap();
    let mut doc = Document {model,meta,path:None,dirty:true};
    let dir = tempfile::tempdir().unwrap();
    library::save(dir.path(), &mut doc).unwrap();
    let (model,meta) = files::load(doc.path.as_deref().unwrap()).unwrap();
    let reopened = Document {model,meta,path:doc.path,dirty:false};
    let tables = business_table::transactions(&reopened).unwrap();
    let index = custom::index(&reopened.model, "Fuel").unwrap();
    let width = reopened.model.get_model().workbook.worksheets[index as usize].dimension().max_column;
    let headers: Vec<_> = (1..=width).map(|c| reopened.model.get_formatted_cell_value(index, 3, c).unwrap()).collect();
    assert!(headers.iter().all(|h| !h.to_lowercase().contains("tax") && !h.to_lowercase().contains("deduct")));
    for (label,expected) in [("Distance",300.0),("Fuel economy",10.0)] {
        let col = headers.iter().position(|h| h == label).unwrap() as i32 + 1;
        assert_eq!(reopened.model.get_formatted_cell_value(index, 4, col).unwrap().parse::<f64>().unwrap(), expected);
    }
    assert!(tables.get("Fuel").is_none());
    assert!(!tables["Income"]["headers"].as_array().unwrap().iter().any(|h| h=="Tax" || h=="Invoice number"));
}
