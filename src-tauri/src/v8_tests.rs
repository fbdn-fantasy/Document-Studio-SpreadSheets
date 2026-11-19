use super::*;
use crate::finance::{Business, Record, Vehicle};

#[test]
fn independent_custom_sheets_keep_formulas_and_names_after_reopening() {
    let (mut model, mut meta) = templates::create("Blank workbook", None).unwrap();
    meta.template = "Custom Template".into();
    meta.settings["custom_template"] = json!({"name":"School and home","sheets":{
        "Gradebook":{"enabled":true,"columns":{}},
        "Monthly Personal Budget":{"enabled":true,"columns":{}},
        "Monthly Budget Summary":{"enabled":true,"columns":{}}
    }});
    assert!(!custom::needs_business(&meta.settings["custom_template"]));
    custom::apply_selection(&mut model, &meta).unwrap();
    let gradebook = custom::index(&model,"Gradebook").unwrap();
    model.set_user_input(gradebook,4,1,"Maya").unwrap();
    let mut doc = Document {model,meta,path:None,dirty:true};
    let folder = tempfile::tempdir().unwrap();
    library::save(folder.path(), &mut doc).unwrap();
    let path = doc.path.as_deref().unwrap();
    assert!(path.ends_with("School and home.xlsx"));
    let (loaded,meta) = files::load(path).unwrap();
    assert!(meta.business.is_none());
    assert_eq!(loaded.get_worksheets_properties().len(),3);
    let budget = custom::index(&loaded,"Monthly Budget Summary").unwrap();
    assert!(loaded.get_cell_content(budget,4,2).unwrap().contains("Monthly Personal Budget"));
    assert_eq!(loaded.get_formatted_cell_value(gradebook,4,1).unwrap(),"Maya");
}

#[test]
fn transaction_description_switches_hide_and_restore_recorded_text() {
    let b = Business {company_id:"v8".into(),name:"Fleet".into(),country:"South Africa".into(),period:"2026-11".into(),currency:"ZAR".into(),year_start:1,..Default::default()};
    let (mut model,mut meta)=templates::create("Custom Template",Some(b)).unwrap();
    for (category,sheet) in [("Fuel","Fuel"),("Expense","Expenses"),("Vehicle repairs","Vehicle Repairs")] {
        meta.settings["custom_template"]["sheets"][sheet]=json!({"enabled":true,"columns":{"Description":false}});
        meta.records.push(Record {id:sheet.into(),category:category.into(),revision:1,period:"2026-11".into(),
            amount:"0".into(),description:"=literal customer note".into(), vehicle:Vehicle::default(),..Default::default()});
    }
    fn headers(model: &UserModel, sheet: u32) -> Vec<String> {
        let width = model.get_model().workbook.worksheets[sheet as usize].dimension().max_column;
        (1..=width).map(|c| model.get_formatted_cell_value(sheet,3,c).unwrap()).collect()
    }
    custom::apply_selection(&mut model,&meta).unwrap();
    for name in ["Fuel","Expenses","Vehicle Repairs"] {
        let sheet = custom::index(&model,name).unwrap();
        assert!(!headers(&model,sheet).iter().any(|h| h=="Description"));
        meta.settings["custom_template"]["sheets"][name]["columns"]["Description"]=json!(true);
    }
    custom::apply_selection(&mut model,&meta).unwrap();
    for name in ["Fuel","Expenses","Vehicle Repairs"] {
        let sheet = custom::index(&model,name).unwrap();
        let column=headers(&model,sheet).iter().position(|h| h=="Description").unwrap() as i32+1;
        assert_eq!(model.get_formatted_cell_value(sheet,4,column).unwrap(),"=literal customer note");
    }
}

