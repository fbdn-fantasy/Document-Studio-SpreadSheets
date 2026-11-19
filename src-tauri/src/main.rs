#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod files;
mod library;
mod business_table;
mod finance;
mod live;
mod templates;
mod custom;
#[cfg(test)]
mod verification;
#[cfg(test)]
mod live_tests;
#[cfg(test)]
mod v4_tests;
#[cfg(test)]
mod v5_tests;
#[cfg(test)]
mod v6_tests;
#[cfg(test)]
mod v8_tests;
use finance::Metadata;
use ironcalc::base::{expressions::types::Area, UserModel};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::Mutex};
use tauri::{Emitter, Manager, State};

struct Document {
    model: UserModel<'static>,
    meta: Metadata,
    path: Option<String>,
    dirty: bool,
}
struct Studio {
    docs: BTreeMap<String, Document>,
    db: Connection,
    library: std::path::PathBuf,
    recoverable: Vec<Value>,
}
type Shared = Mutex<Studio>;

#[tauri::command]
async fn open_support_page() -> Result<(), String> {
    const URL: &str = "https://ko-fi.com/forbidden_fantasy";
    tauri::async_runtime::spawn_blocking(|| open::that_detached(URL))
        .await
        .map_err(err)?
        .map_err(|e| format!("Couldn't open your browser. You can visit {URL} directly. {e}"))
}

#[tauri::command]
async fn native_print(window: tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(windows)]
    {
        use webview2_com::Microsoft::Web::WebView2::Win32::{
            ICoreWebView2_16, COREWEBVIEW2_PRINT_DIALOG_KIND_SYSTEM,
        };
        use windows_core::Interface;
        let (tx, rx) = std::sync::mpsc::channel();
        window
            .with_webview(move |webview| {
                let result = (|| unsafe {
                    webview
                        .controller()
                        .CoreWebView2()?
                        .cast::<ICoreWebView2_16>()?
                        .ShowPrintUI(COREWEBVIEW2_PRINT_DIALOG_KIND_SYSTEM)
                })();
                let _ = tx.send(result.map_err(err));
            })
            .map_err(err)?;
        rx.recv().map_err(err)?
    }
    #[cfg(not(windows))]
    {
        window.print().map_err(err)
    }
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn same_file(a: &str, b: &str) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => {
            if cfg!(windows) {
                a.to_string_lossy()
                    .eq_ignore_ascii_case(&b.to_string_lossy())
            } else {
                a == b
            }
        }
        _ => a == b,
    }
}
fn val<'a>(a: &'a Value, k: &str) -> &'a str {
    a[k].as_str().unwrap_or("")
}
fn int(a: &Value, k: &str, default: i64) -> i64 {
    a[k].as_i64().unwrap_or(default)
}
fn info(id: &str, d: &Document) -> Value {
    json!({"id":id,"name":d.path.as_deref().and_then(|p|std::path::Path::new(p).file_name()).map(|s|s.to_string_lossy().to_string()).unwrap_or(d.meta.template.clone()),"path":d.path,"dirty":d.dirty,"sheets":d.model.get_worksheets_properties(),"metadata":d.meta,"canUndo":d.model.can_undo(),"canRedo":d.model.can_redo()})
}
fn checkpoint(s: &Studio, id: &str) -> Result<(), String> {
    let d = s.docs.get(id).ok_or("Workbook not found")?;
    s.db.execute(
        "INSERT OR REPLACE INTO recovery(id,model,metadata,path,dirty) VALUES(?1,?2,?3,?4,?5)",
        params![
            id,
            d.model.to_bytes(),
            serde_json::to_string(&d.meta).map_err(err)?,
            d.path,
            d.dirty
        ],
    )
    .map_err(err)?;
    Ok(())
}
#[tauri::command]
fn startup(state: State<Shared>) -> Result<Value, String> {
    let s = state.lock().map_err(err)?;
    let settings =
        s.db.query_row(
            "SELECT value FROM settings WHERE key='preferences'",
            [],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or("{}".into());
    let mut q =
        s.db.prepare("SELECT path FROM recent ORDER BY opened DESC LIMIT 15")
            .map_err(err)?;
    let recent = q
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    Ok(
        json!({"settings":serde_json::from_str::<Value>(&settings).unwrap_or(json!({})),"recovery":s.recoverable,"recent":recent,"templates":templates::NAMES,"launchFiles":std::env::args().skip(1).filter(|x|x.ends_with(".xlsx")||x.ends_with(".ods")).collect::<Vec<_>>()}),
    )
}
#[tauri::command]
fn add_worksheets(state: State<Shared>,id:String,config:Value,business:Option<finance::Business>)->Result<Value,String>{
    let mut s=state.lock().map_err(err)?;let d=s.docs.get_mut(&id).ok_or("Workbook not found")?;
    let mut meta=d.meta.clone();let mut model=UserModel::from_bytes(&d.model.to_bytes(),"en")?;
    if let Some(b)=&meta.business {if meta.closed_periods.contains(&b.period){return Err("This period is closed".into());}}
    if meta.business.is_none() && custom::needs_business(&config) { meta.business=business; }
    for sheet in d.model.get_worksheets_properties(){if config["sheets"][&sheet.name]["enabled"]!=true{return Err("Keep existing worksheets selected when adding sheets".into());}}
    if d.meta.business.is_some()&&!d.meta.settings["custom_template"].is_object(){custom::capture_stock(&model,&mut meta)?;}
    meta.settings["custom_template"]=config;
    live::sync_records(&mut meta)?;finance::validate(&meta)?;custom::apply_selection(&mut model,&meta)?;
    meta.audit.push("Added selected worksheets".into());d.model=model;d.meta=meta;d.dirty=true;checkpoint(&s,&id)?;Ok(info(&id,&s.docs[&id]))
}
#[tauri::command]
fn template_catalog() -> Result<Value,String> { custom::catalog() }
#[tauri::command]
fn preferences(state: State<Shared>, value: Value) -> Result<(), String> {
    state
        .lock()
        .map_err(err)?
        .db
        .execute(
            "INSERT OR REPLACE INTO settings(key,value) VALUES('preferences',?1)",
            [value.to_string()],
        )
        .map_err(err)?;
    Ok(())
}
#[tauri::command]
fn create_workbook(
    state: State<Shared>,
    template: String,
    business: Option<finance::Business>,
    source_id: Option<String>,
    custom_template: Option<Value>,
) -> Result<Value, String> {
    let standalone = template == "Custom Template" && business.is_none() && custom_template.is_some();
    let seed = if standalone { "Blank workbook" } else { &template };
    let (mut model, mut meta) = templates::create(seed, business)?;
    if standalone { meta.template = template; }
    if let Some(source_id) = source_id {
        let s = state.lock().map_err(err)?;
        let source = s.docs.get(&source_id).ok_or("Source workbook not found")?;
        meta = live::rollover(&source.meta, meta.business.clone().ok_or("Business settings required")?)?;
        templates::sync(&mut model, &meta)?;
    }
    if let Some(config)=custom_template { meta.settings["custom_template"]=config; custom::apply_selection(&mut model,&meta)?; }
    finance::validate(&meta)?;
    let id = uuid::Uuid::new_v4().to_string();
    let mut s = state.lock().map_err(err)?;
    let mut doc = Document { model, meta, path: None, dirty: true };
    library::save(&s.library, &mut doc)?;
    s.docs.insert(id.clone(), doc);
    checkpoint(&s, &id)?;
    Ok(info(&id, &s.docs[&id]))
}
#[tauri::command]
fn open_workbook(state: State<Shared>, path: String) -> Result<Value, String> {
    let (model, meta) = files::load(&path)?;
    let mut s = state.lock().map_err(err)?;
    if let Some((id, d)) = s
        .docs
        .iter()
        .find(|(_, d)| d.path.as_deref().is_some_and(|p| same_file(p, &path)))
    {
        return Ok(info(id, d));
    }
    let id = uuid::Uuid::new_v4().to_string();
    s.db.execute(
        "INSERT OR REPLACE INTO recent(path,opened) VALUES(?1,datetime('now'))",
        [&path],
    )
    .map_err(err)?;
    s.docs.insert(
        id.clone(),
        Document {
            model,
            meta,
            path: Some(path),
            dirty: false,
        },
    );
    checkpoint(&s, &id)?;
    Ok(info(&id, &s.docs[&id]))
}
#[tauri::command]
fn list_workbooks(state: State<Shared>) -> Result<Vec<Value>, String> {
    let s = state.lock().map_err(err)?;
    let mut items = library::list(&s.library)?;
    let mut query = s.db.prepare("SELECT path FROM recent ORDER BY opened DESC").map_err(err)?;
    for item in query.query_map([], |row| row.get::<_, String>(0)).map_err(err)? {
        let path = item.map_err(err)?;
        let file = std::path::Path::new(&path);
        if !file.is_file() || file.parent() == Some(&s.library) { continue; }
        items.push(json!({"path":file,"name":file.file_name().unwrap_or_default().to_string_lossy(),"external":true}));
    }
    Ok(items)
}

#[tauri::command]
fn save_workbook(state: State<Shared>, id: String, path: Option<String>) -> Result<Value, String> {
    let mut s = state.lock().map_err(err)?;
    if let Some(destination) = &path {
        if s.docs.iter().any(|(other, d)| other != &id && d.path.as_deref().is_some_and(|p| same_file(p, destination))) {
            return Err("This destination is already open in another workbook.".into());
        }
    }
    let directory = s.library.clone();
    let d = s.docs.get_mut(&id).ok_or("Workbook not found")?;
    if let Some(destination) = &path {
        if !d.path.as_deref().is_some_and(|p| same_file(p, destination) && std::path::Path::new(p).parent() == Some(&directory)) {
            files::save(destination, &d.model, &d.meta)?;
        }
    }
    library::save(&directory, d)?;
    checkpoint(&s, &id)?;
    Ok(info(&id, &s.docs[&id]))
}
#[tauri::command]
fn close_workbook(state: State<Shared>, id: String) -> Result<(), String> {
    let mut s = state.lock().map_err(err)?;
    s.db.execute("DELETE FROM recovery WHERE id=?1", [&id])
        .map_err(err)?;
    s.docs.remove(&id);
    Ok(())
}
#[tauri::command]
fn recover(state: State<Shared>, id: String, restore: bool) -> Result<Value, String> {
    let mut s = state.lock().map_err(err)?;
    if !restore {
        s.db.execute("DELETE FROM recovery WHERE id=?1", [&id])
            .map_err(err)?;
        s.recoverable.retain(|v| v["id"] != id);
        return Ok(Value::Null);
    }
    let (bytes, text, path): (Vec<u8>, String, Option<String>) =
        s.db.query_row(
            "SELECT model,metadata,path FROM recovery WHERE id=?1",
            [&id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(err)?;
    let mut model = UserModel::from_bytes(&bytes, "en")?;
    let mut meta = serde_json::from_str(&text).map_err(err)?;
    live::sync_records(&mut meta)?;
    templates::sync(&mut model, &meta)?;
    s.docs.insert(
        id.clone(),
        Document {
            model,
            meta,
            path,
            dirty: true,
        },
    );
    s.recoverable.retain(|v| v["id"] != id);
    Ok(info(&id, &s.docs[&id]))
}
#[tauri::command]
fn viewport(
    state: State<Shared>,
    id: String,
    sheet: u32,
    row: i32,
    column: i32,
    height: i32,
    width: i32,
) -> Result<Value, String> {
    if row < 1 || column < 1 || height < 1 || width < 1 || height > 150 || width > 80 {
        return Err("Invalid viewport".into());
    }
    let s = state.lock().map_err(err)?;
    let d = s.docs.get(&id).ok_or("Workbook not found")?;
    let mut cells = vec![];
    for r in row..=(row + height - 1).min(1048576) {
        for c in column..=(column + width - 1).min(16384) {
            let text = d.model.get_formatted_cell_value(sheet, r, c)?;
            let input = d.model.get_cell_content(sheet, r, c)?;
            let style = d.model.get_cell_style(sheet, r, c)?;
            cells.push(json!({"r":r,"c":c,"text":text,"input":input,"style":style,"fill":d.model.resolve_color(&style.fill.color),"color":d.model.resolve_color(&style.font.color)}));
        }
    }
    let ws = d
        .model
        .get_model()
        .workbook
        .worksheets
        .get(sheet as usize)
        .ok_or("Sheet not found")?;
    let dim = ws.dimension();
    let columns=ws.cols.iter().filter(|c|c.custom_width).map(|c|json!({"start":c.min,"end":c.max,"size":d.model.get_column_width(sheet,c.min).unwrap_or(130.0)})).collect::<Vec<_>>();
    let rows=ws.rows.iter().filter(|r|r.custom_height).map(|r|json!({"start":r.r,"end":r.r,"size":d.model.get_row_height(sheet,r.r).unwrap_or(30.0)})).collect::<Vec<_>>();
    Ok(
        json!({"cells":cells,"rows":dim.max_row,"cols":dim.max_column,"merges":ws.merged_cells,"columnSizes":columns,"rowSizes":rows}),
    )
}
#[tauri::command]
fn cell(
    state: State<Shared>,
    id: String,
    sheet: u32,
    row: i32,
    column: i32,
) -> Result<Value, String> {
    let s = state.lock().map_err(err)?;
    let d = s.docs.get(&id).ok_or("Workbook not found")?;
    Ok(
        json!({"input":d.model.get_cell_content(sheet,row,column)?,"text":d.model.get_formatted_cell_value(sheet,row,column)?}),
    )
}
#[tauri::command]
fn copy_cells(state: State<Shared>, id: String, area: Area) -> Result<Value, String> {
    let mut s = state.lock().map_err(err)?;
    let d = s.docs.get_mut(&id).ok_or("Workbook not found")?;
    if (area.width as i64) * (area.height as i64) > 100000 {
        return Err("Copy at most 100,000 cells".into());
    }
    d.model.set_selected_sheet(area.sheet)?;
    d.model.set_selected_cell(area.row, area.column)?;
    d.model.set_selected_range(
        area.row,
        area.column,
        area.row + area.height - 1,
        area.column + area.width - 1,
    )?;
    serde_json::to_value(d.model.copy_to_clipboard()?).map_err(err)
}
#[tauri::command]
fn action(state: State<Shared>, id: String, kind: String, args: Value) -> Result<Value, String> {
    let mut s = state.lock().map_err(err)?;
    let d = s.docs.get_mut(&id).ok_or("Workbook not found")?;
    let sheet = int(&args, "sheet", 0) as u32;
    let row = int(&args, "row", 1) as i32;
    let column = int(&args, "column", 1) as i32;
    let area = Area {
        sheet,
        row,
        column,
        width: int(&args, "width", 1) as i32,
        height: int(&args, "height", 1) as i32,
    };
    if area.width < 1 || area.height < 1 || (area.width as i64) * (area.height as i64) > 100000 {
        return Err("Select at most 100,000 cells per operation".into());
    }
    let sheet_name=d.model.get_worksheets_properties().get(sheet as usize).ok_or("Worksheet not found")?.name.clone();
    let editable_stock=sheet_name=="Inventory" && !d.meta.settings["custom_template"].is_object();
    let generic_sheet=!custom::SHEETS.contains(&sheet_name.as_str());
    if d.meta.business.is_some()
        && ((!editable_stock && !generic_sheet)
            || !matches!(
                kind.as_str(),
                "set"
                    | "paste"
                    | "clear"
                    | "style"
                    | "fillDown"
                    | "fillRight"
                    | "columnWidth"
                    | "rowHeight"
            ))
    {
        return Err("Linked sheets are calculated from the Business panel. Use its transaction, payroll or inventory menus. Independent spreadsheet templates can be edited directly.".into());
    }
    let original = d.model.to_bytes();
    let result = (|| -> Result<(), String> {
        match kind.as_str() {
            "set" => d
                .model
                .set_user_input(sheet, row, column, val(&args, "value"))?,
            "pasteInternal" => {
                d.model.set_selected_sheet(sheet)?;
                d.model.set_selected_cell(row, column)?;
                let clip = &args["clipboard"];
                let source_range = serde_json::from_value(clip["range"].clone()).map_err(err)?;
                let data = serde_json::from_value(clip["data"].clone()).map_err(err)?;
                d.model.paste_from_clipboard(
                    clip["sheet"].as_u64().ok_or("Invalid source sheet")? as u32,
                    source_range,
                    &data,
                    args["cut"].as_bool().unwrap_or(false),
                )?;
            }
            "paste" => {
                let lines = val(&args, "value")
                    .trim_end_matches(['\r', '\n'])
                    .split('\n')
                    .collect::<Vec<_>>();
                if lines.len() > 10000 {
                    return Err("Paste at most 10,000 rows at a time".into());
                }
                d.model.pause_evaluation();
                for (i, line) in lines.iter().enumerate() {
                    for (j, text) in line.trim_end_matches('\r').split('\t').enumerate() {
                        d.model
                            .set_user_input(sheet, row + i as i32, column + j as i32, text)?;
                    }
                }
                d.model.resume_evaluation();
            }
            "clear" => d.model.range_clear_contents(&area)?,
            "style" => {
                d.model
                    .update_range_style(&area, val(&args, "path"), val(&args, "value"))?
            }
            "undo" => d.model.undo()?,
            "redo" => d.model.redo()?,
            "addSheet" => d.model.new_sheet()?,
            "renameSheet" => d.model.rename_sheet(sheet, val(&args, "value"))?,
            "duplicateSheet" => d.model.duplicate_sheet(sheet)?,
            "deleteSheet" => d.model.delete_sheet(sheet)?,
            "moveSheet" => d.model.move_sheet(sheet, int(&args, "to", 0) as u32)?,
            "hideSheet" => d.model.hide_sheet(sheet)?,
            "showSheet" => d.model.unhide_sheet(sheet)?,
            "insertRows" => d.model.insert_rows(sheet, row, area.height)?,
            "deleteRows" => d.model.delete_rows(sheet, row, area.height)?,
            "insertColumns" => d.model.insert_columns(sheet, column, area.width)?,
            "deleteColumns" => d.model.delete_columns(sheet, column, area.width)?,
            "columnWidth" => d.model.set_columns_width(
                sheet,
                column,
                column + area.width - 1,
                args["value"].as_f64().ok_or("Invalid width")?,
            )?,
            "rowHeight" => d.model.set_rows_height(
                sheet,
                row,
                row + area.height - 1,
                args["value"].as_f64().ok_or("Invalid height")?,
            )?,
            "fillDown" => d
                .model
                .auto_fill_rows(&Area { height: 1, ..area }, row + area.height - 1)?,
            "fillRight" => d
                .model
                .auto_fill_columns(&Area { width: 1, ..area }, column + area.width - 1)?,
            "merge" => d.model.merge_cells(&area)?,
            "unmerge" => d.model.unmerge_cells(&area)?,
            "defineName" => {
                let name = d
                    .model
                    .get_worksheets_properties()
                    .get(sheet as usize)
                    .ok_or("Unknown sheet")?
                    .name
                    .replace('\'', "''");
                let reference = format!(
                    "'{}'!${}${}:${}${}",
                    name,
                    templates::col(column),
                    row,
                    templates::col(column + area.width - 1),
                    row + area.height - 1
                );
                d.model
                    .new_defined_name(ironcalc::base::DefinedNameEntry::new(
                        val(&args, "value"),
                        &reference,
                    ))?;
            }
            "replace" => {
                let find = val(&args, "find");
                if find.is_empty() {
                    return Err("Enter text to find".into());
                }
                d.model.pause_evaluation();
                for r in row..row + area.height {
                    for c in column..column + area.width {
                        let v = d.model.get_cell_content(sheet, r, c)?;
                        if v.contains(find) {
                            d.model.set_user_input(
                                sheet,
                                r,
                                c,
                                &v.replace(find, val(&args, "value")),
                            )?;
                        }
                    }
                }
                d.model.resume_evaluation();
            }
            _ => return Err("Unknown edit operation".into()),
        };
        Ok(())
    })();
    if let Err(e) = result {
        d.model = UserModel::from_bytes(&original, "en")?;
        return Err(e);
    }
    d.dirty = true;
    checkpoint(&s, &id)?;
    Ok(info(&id, &s.docs[&id]))
}
#[tauri::command]
fn preview_formula(
    state: State<Shared>,
    id: String,
    sheet: u32,
    row: i32,
    column: i32,
    formula: String,
) -> Result<Value, String> {
    let s = state.lock().map_err(err)?;
    let d = s.docs.get(&id).ok_or("Workbook not found")?;
    let mut scratch = UserModel::from_bytes(&d.model.to_bytes(), "en")?;
    scratch.set_user_input(sheet, row, column, &formula)?;
    Ok(json!({"value":scratch.get_formatted_cell_value(sheet,row,column)?}))
}
#[tauri::command]
fn update_business(state: State<Shared>, id: String, mut metadata: Metadata) -> Result<Value, String> {
    let mut s = state.lock().map_err(err)?;
    let d = s.docs.get_mut(&id).ok_or("Workbook not found")?;
    if metadata.workbook_id != d.meta.workbook_id {
        return Err("Workbook identity cannot change".into());
    }
    if !metadata.closed_periods.is_superset(&d.meta.closed_periods) {
        return Err("Closed periods cannot be reopened by replacing metadata".into());
    }
    if let (Some(old), Some(new)) = (&d.meta.business, &metadata.business) {
        if old.company_id != new.company_id || old.period != new.period || old.currency != new.currency || old.country != new.country || old.accounting != new.accounting {
            return Err("Company identity, currency and period cannot change. Use Start a new month.".into());
        }
        if d.meta.closed_periods.contains(&old.period) && serde_json::to_value(&metadata).map_err(err)? != serde_json::to_value(&d.meta).map_err(err)? { return Err("This period is closed".into()); }
    } else { return Err("Business settings are required".into()); }
    for old in &d.meta.records {
        let Some(next) = metadata.records.iter().find(|r| r.id == old.id) else {
            if !old.payments.is_empty() || !old.managed_key.is_empty() || old.period != d.meta.business.as_ref().unwrap().period { return Err("Records with payment history or generated obligations cannot be deleted".into()); }
            continue;
        };
        if d.meta.closed_periods.contains(&old.period) && old != next {
            return Err("Closed-period records cannot be changed".into());
        }
        if old.period != next.period || old.managed_key != next.managed_key { return Err("Record identity cannot change".into()); }
        if old.period != d.meta.business.as_ref().unwrap().period && (old.amount != next.amount || old.party != next.party || old.calculation != next.calculation || old.category != next.category) { return Err("Prior-period financial inputs cannot change here".into()); }
        for payment in &old.payments {
            if !next.payments.contains(payment) {
                return Err("Payment history cannot be overwritten. Add a reversal.".into());
            }
        }
    }
    if metadata.settings["custom_template"].is_object() && !d.meta.settings["custom_template"].is_object(){custom::capture_stock(&d.model,&mut metadata)?;}
    live::sync_records(&mut metadata)?;
    finance::validate(&metadata)?;
    let before = d.model.to_bytes();
    if let Err(e) = templates::sync(&mut d.model, &metadata) {
        d.model = UserModel::from_bytes(&before, "en")?;
        return Err(e);
    }
    d.meta = metadata;
    d.dirty = true;
    checkpoint(&s, &id)?;
    Ok(info(&id, &s.docs[&id]))
}
#[tauri::command]
fn financial_view(
    state: State<Shared>,
    id: String,
    from: Option<String>,
    to: Option<String>,
) -> Result<Value, String> {
    let s = state.lock().map_err(err)?;
    let d = s.docs.get(&id).ok_or("Workbook not found")?;
    let period = d
        .meta
        .business
        .as_ref()
        .ok_or("Configure a business first")?
        .period
        .as_str();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let balances = d
        .meta
        .records
        .iter()
        .map(|r| finance::balance(r, &today))
        .collect::<Result<Vec<_>, _>>()?;
    let tables = business_table::transactions(d)?;
    let mut fuel = BTreeMap::new();
    let country = &d.meta.business.as_ref().unwrap().country;
    for record in d.meta.records.iter().filter(|r| r.category == "Fuel") {
        let (distance, economy) = custom::fuel_stats(record, country)?;
        let (_, distance_unit, economy_unit) = custom::units(country, &record.vehicle.units);
        fuel.insert(&record.id, json!({"distance":distance,"economy":economy,"distanceUnit":distance_unit,"economyUnit":economy_unit}));
    }
    let payroll = d
        .meta
        .employees
        .iter()
        .map(|e| finance::payroll(e, period))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(
        json!({"tables":tables,"fuel":fuel,"summary":finance::summary(&d.meta,from.as_deref().unwrap_or(period),to.as_deref().unwrap_or(period))?,"balances":balances,"payroll":payroll,"taxes":live::tax_view(&d.meta)?,"days":finance::days_in_month(period)?}),
    )
}
#[tauri::command]
fn set_paid(state: State<Shared>, id: String, record_id: String, paid: bool) -> Result<Value, String> {
    let mut s = state.lock().map_err(err)?;
    let d = s.docs.get_mut(&id).ok_or("Workbook not found")?;
    let mut meta = d.meta.clone();
    let r = meta.records.iter_mut().find(|r| r.id == record_id).ok_or("Record not found")?;
    if r.category == "Fuel" && r.pricing.enabled { return Err("Fuel purchases are paid in full. Edit the purchase to correct its price.".into()); }
    if meta.closed_periods.contains(&r.period) { return Err("This period is closed".into()); }
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    if paid {
        let remaining = finance::balance(r, &date)?.outstanding;
        if finance::money(&remaining)? > rust_decimal::Decimal::ZERO {
            r.payments.push(finance::Payment { id: uuid::Uuid::new_v4().to_string(), date: date.clone(), amount: remaining, reference: "Marked paid".into(), method: "Manual confirmation".into(), reversal_of: None, notes: "Payment date recorded automatically".into() });
        }
    } else {
        let active = r.payments.iter().filter(|p| p.reversal_of.is_none() && !r.payments.iter().any(|x| x.reversal_of.as_ref() == Some(&p.id))).cloned().collect::<Vec<_>>();
        for p in active {
            r.payments.push(finance::Payment { id: uuid::Uuid::new_v4().to_string(), date: date.clone(), amount: (-finance::money(&p.amount)?).to_string(), reference: "Marked unpaid".into(), method: "Correction".into(), reversal_of: Some(p.id), notes: "Original payment retained in history".into() });
        }
    }
    meta.audit.push(format!("{}: {} marked {}", date, record_id, if paid { "paid" } else { "unpaid" }));
    finance::validate(&meta)?;
    let before = d.model.to_bytes();
    if let Err(e) = templates::sync(&mut d.model, &meta) { d.model = UserModel::from_bytes(&before, "en")?; return Err(e); }
    d.meta = meta; d.dirty = true;
    checkpoint(&s, &id)?;
    Ok(info(&id, &s.docs[&id]))
}
#[tauri::command]
fn inventory_view(state: State<Shared>, id: String) -> Result<Value, String> {
    let s = state.lock().map_err(err)?;
    let d = s.docs.get(&id).ok_or("Workbook not found")?;
    if d.meta.business.is_none() { return Err("Business workbook required".into()); }
    let mut model=UserModel::from_bytes(&d.model.to_bytes(),"en")?; custom::expand(&mut model)?; custom::restore_stock(&mut model,&d.meta)?;
    let mut rows = vec![];
    for row in 4..=103 {
        let product = model.get_formatted_cell_value(8,row,1)?;
        if product.is_empty() { continue; }
        rows.push(json!({"row":row,"product":product,"opening":model.get_cell_content(8,row,2)?,"received":model.get_cell_content(8,row,3)?,"sold":model.get_cell_content(8,row,4)?,"closing":model.get_formatted_cell_value(8,row,5)?,"cost":model.get_cell_content(8,row,6)?,"value":model.get_formatted_cell_value(8,row,7)?}));
    }
    Ok(json!({"rows":rows}))
}
#[tauri::command]
fn save_inventory(state: State<Shared>, id: String, row: i32, product: String, opening: String, received: String, sold: String, cost: String, remove: bool) -> Result<Value, String> {
    let mut s = state.lock().map_err(err)?;
    let d = s.docs.get_mut(&id).ok_or("Workbook not found")?;
    let b = d.meta.business.as_ref().ok_or("Business workbook required")?;
    if d.meta.closed_periods.contains(&b.period) { return Err("This period is closed".into()); }
    let mut model=UserModel::from_bytes(&d.model.to_bytes(),"en")?; custom::expand(&mut model)?; custom::restore_stock(&mut model,&d.meta)?;
    let row = if row == 0 { (4..=103).find(|r| model.get_cell_content(8,*r,1).unwrap_or_default().is_empty()).ok_or("The 100 inventory rows are full")? } else { row };
    if !(4..=103).contains(&row) { return Err("Invalid inventory row".into()); }
    let values = if remove { vec!["".to_string();7] } else {
        if product.trim().is_empty() { return Err("Enter a product name".into()); }
        let o=finance::nonnegative(&opening)?; let r=finance::nonnegative(&received)?; let sold_value=finance::nonnegative(&sold)?;
        if sold_value > o+r { return Err("Sold units exceed available stock".into()); }
        finance::nonnegative(&cost)?;
        vec![format!("'{}",product),o.to_string(),r.to_string(),sold_value.to_string(),format!("=B{row}+C{row}-D{row}"),cost,format!("=E{row}*F{row}")]
    };
    let mut meta=d.meta.clone();
    for(c,v)in values.iter().enumerate(){model.set_user_input(8,row,c as i32+1,v)?;}
    custom::capture_stock(&model,&mut meta)?;
    templates::sync(&mut model,&meta)?;
    d.model=model;d.meta=meta;
    d.meta.audit.push(format!("Inventory row {} {}",row,if remove {"removed"} else {"updated"}));
    d.dirty = true; checkpoint(&s,&id)?;
    Ok(info(&id,&s.docs[&id]))
}
#[tauri::command]
fn import_periods(
    state: State<Shared>,
    id: String,
    paths: Vec<String>,
    rollover: bool,
    commit: bool,
) -> Result<Value, String> {
    if paths.len() > 12 {
        return Err("Select at most twelve monthly workbooks".into());
    }
    let mut sources = vec![];
    for path in paths {
        sources.push(files::load(&path)?.1);
    }
    let mut s = state.lock().map_err(err)?;
    let d = s.docs.get_mut(&id).ok_or("Workbook not found")?;
    let mut merged = d.meta.clone();
    for source in &sources {
        merged = finance::merge(&merged, source, rollover)?;
    }
    if !commit {
        return Ok(
            json!({"records":merged.records,"sourcePeriods":sources.iter().map(|m|m.business.as_ref().map(|b|b.period.clone())).collect::<Vec<_>>(),"count":merged.records.len()-d.meta.records.len()}),
        );
    }
    live::sync_records(&mut merged)?;
    let bytes = d.model.to_bytes();
    if let Err(e) = templates::sync(&mut d.model, &merged) {
        d.model = UserModel::from_bytes(&bytes, "en")?;
        return Err(e);
    }
    d.meta = merged;
    d.dirty = true;
    checkpoint(&s, &id)?;
    Ok(info(&id, &s.docs[&id]))
}
#[tauri::command]
fn print_data(
    state: State<Shared>,
    id: String,
    sheet: u32,
    rows: i32,
    columns: i32,
) -> Result<Value, String> {
    if rows < 1 || columns < 1 || rows > 10000 || columns > 100 {
        return Err("Print area must be at most 10,000 rows by 100 columns".into());
    }
    let s = state.lock().map_err(err)?;
    let d = s.docs.get(&id).ok_or("Workbook not found")?;
    let mut data = vec![];
    for r in 1..=rows {
        let mut line = vec![];
        for c in 1..=columns {
            line.push(d.model.get_formatted_cell_value(sheet, r, c)?);
        }
        data.push(line);
    }
    Ok(json!(data))
}
// Visibility belongs to frontend readiness, not saved window state.
#[tauri::command]
fn show_ready_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.show().map_err(err)?;
    window.set_focus().map_err(err)
}

fn main() {
    let app=tauri::Builder::default().plugin(tauri_plugin_single_instance::init(|app,args,_|{if let Some(window)=app.get_webview_window("main"){let _=window.set_focus();}let _=app.emit("open-files",args.into_iter().skip(1).filter(|x|x.ends_with(".xlsx")||x.ends_with(".ods")).collect::<Vec<_>>());})).plugin(tauri_plugin_dialog::init()).plugin(tauri_plugin_window_state::Builder::default()
            .with_filename(std::env::var_os("DSS_TEST_PROFILE")
                .map(|path| std::path::PathBuf::from(path).join(".window-state.json").to_string_lossy().into_owned())
                .unwrap_or_else(|| ".window-state.json".into()))
            .with_state_flags(tauri_plugin_window_state::StateFlags::all() & !tauri_plugin_window_state::StateFlags::VISIBLE)
            .build())
        .setup(|app|{let directory=std::env::var_os("DSS_TEST_PROFILE").map(std::path::PathBuf::from).unwrap_or(app.path().app_data_dir()?);std::fs::create_dir_all(&directory)?;let db=Connection::open(directory.join("studio.sqlite"))?;db.execute_batch("PRAGMA journal_mode=WAL;PRAGMA synchronous=FULL;CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY,value TEXT);CREATE TABLE IF NOT EXISTS recovery(id TEXT PRIMARY KEY,model BLOB,metadata TEXT,path TEXT,dirty INTEGER);CREATE TABLE IF NOT EXISTS recent(path TEXT PRIMARY KEY,opened TEXT);")?;
            let clean=db.query_row("SELECT value FROM settings WHERE key='clean'",[],|r|r.get::<_,String>(0)).unwrap_or("yes".into());let mut recoverable=vec![];
            if clean=="no"{let mut q=db.prepare("SELECT id,path,metadata FROM recovery WHERE dirty=1")?;for row in q.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?,r.get::<_,String>(2)?)))?{let (id,path,meta)=row?;recoverable.push(json!({"id":id,"path":path,"metadata":serde_json::from_str::<Value>(&meta).unwrap_or(Value::Null)}));}}else{db.execute("DELETE FROM recovery",[])?;}
            db.execute("INSERT OR REPLACE INTO settings(key,value) VALUES('clean','no')",[])?;app.manage(Mutex::new(Studio{docs:BTreeMap::new(),db,recoverable,library:directory.join("Workbooks")}));Ok(())})
        .invoke_handler(tauri::generate_handler![show_ready_window,open_support_page,startup,list_workbooks, template_catalog,add_worksheets,preferences,create_workbook,open_workbook,save_workbook,close_workbook,recover,viewport,cell,action,preview_formula,update_business,set_paid,inventory_view,save_inventory,financial_view,import_periods,print_data,native_print,copy_cells]).build(tauri::generate_context!()).expect("Unable to start Document Studio Spreadsheets");
    app.run(|handle, event| {
        if let tauri::RunEvent::Exit = event {
            let state = handle.state::<Shared>();
            if let Ok(s) = state.lock() {
                for id in s.docs.keys() {
                    let _ = s.db.execute("DELETE FROM recovery WHERE id=?1", [id]);
                }
                let clean = if s.recoverable.is_empty() {
                    "yes"
                } else {
                    "no"
                };
                let _ = s.db.execute(
                    "INSERT OR REPLACE INTO settings(key,value) VALUES('clean',?1)",
                    [clean],
                );
            };
        }
    });
}

