use crate::{finance::*,templates};
use ironcalc::base::UserModel;
use serde_json::{Value,json};
pub const SHEETS:&[&str]=&["Overview","Income","Expenses","Payroll","Employer Tax & Contributions","Profit and Loss","Assets","Refunds","Inventory","Settings","Outstanding Balances","Payment History","Employee Tax & Contributions","Timesheets","Fuel","Vehicle Repairs","Payslips"];
const STOCK:&[&str]=&["Product","Opening units","Received","Sold","Closing units","Unit cost","Stock value"];
const SUMMARIES:&[(&str,&str)]=&[("Monthly Budget Summary","Monthly Personal Budget"),("Annual Budget Summary","Annual Budget"),("School Budget Summary","School Budget"),("Project Budget Summary","Project Budget")];
pub fn restore_stock(m:&mut UserModel,meta:&Metadata)->Result<(),String>{
    if let Some(rows)=meta.settings["inventory_data"].as_array(){
        for (i,row) in rows.iter().enumerate(){if i>=100{break;}if let Some(cols)=row.as_array(){for(c,value)in cols.iter().enumerate(){if c<7 {m.set_user_input(8,i as i32+4,c as i32+1,value.as_str().unwrap_or(""))?;}}}}
    }
    Ok(())
}
pub fn capture_stock(m:&UserModel,meta:&mut Metadata)->Result<(),String>{
    let mut rows=vec![];for r in 4..=103 {rows.push((1..=7).map(|c|m.get_cell_content(8,r,c)).collect::<Result<Vec<_>,_>>()?);}
    meta.settings["inventory_data"]=json!(rows);Ok(())
}
pub fn index(m:&UserModel,name:&str)->Option<u32>{m.get_worksheets_properties().iter().position(|s|s.name==name).map(|n|n as u32)}
pub fn expand(m:&mut UserModel)->Result<(),String>{
    for (i,name) in SHEETS.iter().enumerate(){
        let at=if let Some(at)=index(m,name){at}else{m.new_sheet()?;let at=m.get_worksheets_properties().len() as u32-1;m.rename_sheet(at,name)?;at};
        if at!=i as u32 {m.move_sheet(at,i as u32)?;}
    }
    // Inventory inputs remain editable even when optional report columns are omitted.
    for (c,label) in STOCK.iter().enumerate(){
        if m.get_formatted_cell_value(8,3,c as i32+1)?!=*label {m.insert_columns(8,c as i32+1,1)?;m.set_user_input(8,3,c as i32+1,label)?;}
    }
    for row in 4..=103 { m.set_user_input(8,row,5,&format!("=IF(A{row}=\"\",\"\",B{row}+C{row}-D{row})"))?;m.set_user_input(8,row,7,&format!("=IF(A{row}=\"\",\"\",E{row}*F{row})"))?; }
    Ok(())
}
pub fn units(country:&str,override_units:&str)->(&'static str,&'static str,&'static str){
    let code=if override_units.is_empty(){match country{"United States"=>"US","United Kingdom"=>"UK",_=>"Metric"}}else{override_units};
    match code {"US"=>("US gallons","miles","mpg (US)"),"UK"=>("litres","miles","mpg (Imperial)"),"Imperial"=>("Imperial gallons","miles","mpg (Imperial)"),_=>("litres","km","km/L")}
}
pub fn fuel_stats(r:&Record,country:&str)->Result<(String,String),String>{
    match fuel_distance(r)? {None=>Ok((String::new(),String::new())),Some((d,mut e))=>{if r.vehicle.units=="UK" || (r.vehicle.units.is_empty()&&country=="United Kingdom"){e*=rust_decimal::Decimal::new(454609,5);}Ok((d.to_string(),e.round_dp(3).to_string()))}}
}
fn internal(label:&str)->bool{["Record ID","Payment ID","Transaction ID","Reversal of"].contains(&label)}
pub fn includes(meta:&Metadata,sheet:&str,part:&str)->bool{meta.settings["custom_template"]["sheets"][sheet]["columns"][part]!=false}
pub fn needs_business(config: &Value) -> bool {
    SHEETS.iter().any(|name| config["sheets"][*name]["enabled"] == true)
}

pub fn apply_selection(model: &mut UserModel, meta: &Metadata) -> Result<(), String> {
    if meta.business.is_some() { return templates::sync(model, meta); }
    if needs_business(&meta.settings["custom_template"]) {
        return Err("Enter business details for the selected linked worksheets".into());
    }
    project(model, meta)?;
    model.evaluate();
    Ok(())
}
pub fn project(m:&mut UserModel,meta:&Metadata)->Result<(),String>{
    let config=&meta.settings["custom_template"]["sheets"];
    let custom=config.is_object();
    if custom && !config.as_object().unwrap().values().any(|v|v["enabled"]==true){return Err("Choose at least one worksheet".into());}
    if custom {for(summary,base)in SUMMARIES{if config[*summary]["enabled"]==true&&config[*base]["enabled"]!=true{return Err(format!("{summary} also needs {base}"));}}}
    // Generic templates are independent formula sheets, preserved on later ledger refreshes.
    if custom { for name in templates::NAMES.iter().filter(|n|!SHEETS.contains(n)&&!n.contains("Company")&&!["Blank workbook","General Business","Custom Template"].contains(n)){
        if config[*name]["enabled"]!=true || index(m,name).is_some(){continue;}
        let (source,_)=templates::create(name,None)?;
        m.new_sheet()?;let dest=m.get_worksheets_properties().len() as u32-1;m.rename_sheet(dest,name)?;
        let dim=source.get_model().workbook.worksheets[0].dimension();
        for row in 1..=dim.max_row {for col in 1..=dim.max_column {let value=source.get_cell_content(0,row,col)?;if !value.is_empty(){m.set_user_input(dest,row,col,&value)?;}}}
        m.set_columns_width(dest,1,dim.max_column,160.0)?;
    }}
    if custom {for(summary,base)in SUMMARIES{if config[*summary]["enabled"]!=true||index(m,summary).is_some(){continue;}let(source,_)=templates::create(base,None)?;m.new_sheet()?;let dest=m.get_worksheets_properties().len() as u32-1;m.rename_sheet(dest,summary)?;let dim=source.get_model().workbook.worksheets[1].dimension();for r in 1..=dim.max_row{for c in 1..=dim.max_column{let v=source.get_cell_content(1,r,c)?;if !v.is_empty(){m.set_user_input(dest,r,c,&v)?;}}}}}
    for sheet in (0..m.get_worksheets_properties().len() as u32).rev(){
        let name=m.get_worksheets_properties()[sheet as usize].name.clone();
        if custom && config[&name]["enabled"]!=true {m.delete_sheet(sheet)?;continue;}
        let max=m.get_model().workbook.worksheets[sheet as usize].dimension().max_column;
        for c in (1..=max).rev(){let label=m.get_formatted_cell_value(sheet,3,c)?;
            if internal(&label) || (custom && config[&name]["columns"][&label]==false) {
                // Freeze only linked report formulas; independent sheets retain formulas.
                let rows=m.get_model().workbook.worksheets[sheet as usize].dimension().max_row;
                let title=m.get_formatted_cell_value(sheet,1,1)?;
                let note=m.get_formatted_cell_value(sheet,2,1)?;
                let totals:Vec<_>=(4..=rows).filter(|r|m.get_formatted_cell_value(sheet,*r,1).unwrap_or_default()=="TOTAL").collect();
                m.evaluate();
                if SHEETS.contains(&name.as_str()) {for col in 1..=max {for row in 4..=rows{let value=m.get_cell_content(sheet,row,col)?;if value.starts_with('='){let result=m.get_formatted_cell_value(sheet,row,col)?;let result=result.replace(",","");m.set_user_input(sheet,row,col,&result)?;}}}}
                m.delete_columns(sheet,c,1)?;
                if c==1 {m.set_user_input(sheet,1,1,&format!("'{title}"))?;m.set_user_input(sheet,2,1,&format!("'{note}"))?;for row in totals{if internal(&label)||m.get_cell_content(sheet,row,1)?.is_empty(){m.set_user_input(sheet,row,1,"TOTAL")?;}}}
                if !SHEETS.contains(&name.as_str()){for col in 1..max{for row in 4..=rows{let f=m.get_cell_content(sheet,row,col)?;if f.starts_with('=')&&f.contains("#REF!"){m.set_user_input(sheet,row,col,&f.replace("#REF!","0"))?;}}}}
            }
        }
    }
    Ok(())
}
pub fn catalog()->Result<Value,String>{
    let b=Business{company_id:"catalog".into(),name:"Template".into(),country:"South Africa".into(),currency:"ZAR".into(),period:"2026-11".into(),year_start:1,..Default::default()};
    let (m,_)=templates::create("General Business",Some(b))?;
    let mut out=vec![];
    for (s,p) in m.get_worksheets_properties().iter().enumerate(){let mut columns=vec![];let max=m.get_model().workbook.worksheets[s].dimension().max_column;for c in 1..=max{let label=m.get_formatted_cell_value(s as u32,3,c)?;if !label.is_empty(){columns.push(label);}}
        if ["Payroll","Payslips"].contains(&p.name.as_str()){columns.extend(["Employee identifiers".into(),"Individual contributions".into()]);}
        if p.name=="Payslips"{columns.extend(["Gross pay".into(),"Tax".into(),"Total contributions".into(),"Net salary / employer cost".into()]);}
        out.push(json!({"name":p.name,"columns":columns,"live":true}));}
    for name in templates::NAMES.iter().filter(|n|!n.contains("Company")&&!["Blank workbook","General Business","Custom Template"].contains(n)){
        let(m,_)=templates::create(name,None)?;let max=m.get_model().workbook.worksheets[0].dimension().max_column;let columns=(1..=max).map(|c|m.get_formatted_cell_value(0,3,c)).collect::<Result<Vec<_>,_>>()?;out.push(json!({"name":name,"columns":columns,"live":false}));
    }
    for(summary,base)in SUMMARIES{out.push(json!({"name":summary,"columns":["Measure","Amount"],"live":false,"requires":base}));}
    Ok(json!(out))
}
