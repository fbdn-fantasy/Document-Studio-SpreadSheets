use crate::finance::{balance, payroll, summary, Business, Metadata};
use ironcalc::base::{expressions::types::Area, UserModel};
pub const NAMES: &[&str] = &[
    "Blank workbook",
    "Custom Template",
    "Security Company",
    "Retail Company",
    "Service Company",
    "General Business",
    "Monthly Personal Budget",
    "Household Expenses",
    "Savings Tracker",
    "Debt Repayment Tracker",
    "Personal Income Tracker",
    "Gradebook",
    "Attendance Register",
    "Assignment Tracker",
    "Student Performance Report",
    "School Budget",
    "Annual Budget",
    "Project Budget",
    "Inventory Tracker",
    "Project Planner",
    "Work Schedule",
    "Expense Tracker",
    "Financial Forecast",
    "Invoice Tracker",
];
fn heading(m: &mut UserModel, s: u32, title: &str, cols: &[&str]) -> Result<(), String> {
    literal(m, s, 1, 1, title)?;
    m.update_range_style(
        &Area {
            sheet: s,
            row: 1,
            column: 1,
            width: cols.len() as i32,
            height: 1,
        },
        "font.b",
        "true",
    )?;
    m.update_range_style(
        &Area {
            sheet: s,
            row: 1,
            column: 1,
            width: cols.len() as i32,
            height: 1,
        },
        "fill.color",
        "#4C236B",
    )?;
    m.update_range_style(
        &Area {
            sheet: s,
            row: 1,
            column: 1,
            width: cols.len() as i32,
            height: 1,
        },
        "font.color",
        "#FFFFFF",
    )?;
    for (i, c) in cols.iter().enumerate() {
        m.set_user_input(s, 3, i as i32 + 1, c)?;
    }
    m.update_range_style(
        &Area {
            sheet: s,
            row: 3,
            column: 1,
            width: cols.len() as i32,
            height: 1,
        },
        "font.b",
        "true",
    )?;
    m.set_frozen_rows_count(s, 3)?;
    m.set_columns_width(s, 1, cols.len() as i32, 145.0)?;
    for (i, label) in cols.iter().enumerate() {
        let width = (label.chars().count() as f64 * 8.0 + 24.0).clamp(145.0, 260.0);
        m.set_columns_width(s, i as i32 + 1, i as i32 + 1, width)?;
    }
    Ok(())
}
pub fn create(
    name: &str,
    business: Option<Business>,
) -> Result<(UserModel<'static>, Metadata), String> {
    if !NAMES.contains(&name) {
        return Err("Unknown template".into());
    }
    let mut m = UserModel::new_empty("Workbook", "en", "UTC", "en")?;
    let mut meta = Metadata {
        template: name.into(),
        business,
        ..Default::default()
    };
    if name == "Blank workbook" {
        return Ok((m, meta));
    }
    m.pause_evaluation();
    if meta.business.is_some() {
        let sheets = [
            "Overview",
            "Income",
            "Expenses",
            "Payroll",
            "Employer Tax & Contributions",
            "Profit and Loss",
            "Assets",
            "Refunds",
            "Inventory",
            "Settings",
            "Outstanding Balances",
            "Payment History",
            "Employee Tax & Contributions",
            "Timesheets",
        ];
        for (i, s) in sheets.iter().enumerate() {
            if i > 0 {
                m.new_sheet()?;
            }
            m.rename_sheet(i as u32, s)?;
        }
        for i in [1, 2, 4, 6, 7] {
            heading(
                &mut m,
                i,
                sheets[i as usize],
                &[
                    "Record ID",
                    "Period",
                    "Customer / recipient",
                    "Description",
                    "Amount",
                    "Paid",
                    "Outstanding",
                    "Due date",
                    "Status",
                ],
            )?;
        }
        heading(
            &mut m,
            8,
            "Inventory",
            &[
                "Product",
                "Opening units",
                "Received",
                "Sold",
                "Closing units",
                "Unit cost",
                "Stock value",
            ],
        )?;
        for r in 4..=103 {
            m.set_user_input(8, r, 5, &format!("=IF(A{r}=\"\",\"\",B{r}+C{r}-D{r})"))?;
            m.set_user_input(8, r, 7, &format!("=IF(A{r}=\"\",\"\",E{r}*F{r})"))?;
        }
        sync(&mut m, &meta)?;
    } else {
        m.rename_sheet(0, &name.chars().take(31).collect::<String>())?;
        let (cols,formulas):(Vec<&str>,Vec<(i32,&str)>)=match name{
            "Gradebook"|"Student Performance Report"=>(vec!["Student","Assessment 1","Assessment 2","Assessment 3","Average","Result"],vec![(5,"=IF(A{r}=\"\",\"\",AVERAGE(B{r}:D{r}))"),(6,"=IF(A{r}=\"\",\"\",IF(E{r}>=50,\"Pass\",\"Review\"))")]),
            "Attendance Register"=>(vec!["Student / employee","Days present","Days absent","Total days","Attendance %"],vec![(4,"=IF(A{r}=\"\",\"\",SUM(B{r}:C{r}))"),(5,"=IFERROR(B{r}/D{r},0)")]),
            "Savings Tracker"=>(vec!["Goal","Target","Saved","Remaining","Progress %"],vec![(4,"=IF(A{r}=\"\",\"\",MAX(0,B{r}-C{r}))"),(5,"=IFERROR(C{r}/B{r},0)")]),
            "Debt Repayment Tracker"=>(vec!["Creditor","Opening balance","Interest / charges","Paid","Remaining"],vec![(5,"=IF(A{r}=\"\",\"\",B{r}+C{r}-D{r})")]),
            "Inventory Tracker"=>(vec!["Product","Opening units","Received","Sold","Closing units","Unit cost","Stock value"],vec![(5,"=IF(A{r}=\"\",\"\",B{r}+C{r}-D{r})"),(7,"=IF(A{r}=\"\",\"\",E{r}*F{r})")]),
            "Invoice Tracker"|"Personal Income Tracker"=>(vec!["Reference","Customer / source","Amount due","Amount received","Outstanding","Due date"],vec![(5,"=IF(A{r}=\"\",\"\",C{r}-D{r})")]),
            "Assignment Tracker"|"Project Planner"=>(vec!["Task / assignment","Owner","Start date","Due date","Completed date","Status"],vec![(6,"=IF(A{r}=\"\",\"\",IF(E{r}>0,\"Complete\",IF(D{r}<TODAY(),\"Overdue\",\"In progress\")))")]),
            "Work Schedule"=>(vec!["Employee","Date","Start time","End time","Break (hours)","Hours"],vec![(6,"=IF(A{r}=\"\",\"\",MOD(D{r}-C{r},1)*24-E{r})")]),
            "Financial Forecast"=>(vec!["Period","Projected income","Operating costs","Payroll","Projected profit"],vec![(5,"=IF(A{r}=\"\",\"\",B{r}-C{r}-D{r})")]),
            _=>(vec!["Category","Description","Budget","Actual","Remaining"],vec![(5,"=IF(A{r}=\"\",\"\",C{r}-D{r})")]),
        };
        heading(&mut m, 0, name, &cols)?;
        m.set_user_input(
            0,
            2,
            1,
            "Enter your data below. Formula columns calculate automatically; 100 prepared rows.",
        )?;
        for r in 4..=103 {
            for (c, f) in &formulas {
                m.set_user_input(0, r, *c, &f.replace("{r}", &r.to_string()))?;
            }
        }
        m.set_user_input(0, 105, 1, "Total / average")?;
        for (c, _) in formulas {
            m.set_user_input(0, 105, c, &format!("=SUM({}4:{}103)", col(c), col(c)))?;
        }
        if name == "Monthly Personal Budget"
            || name == "Annual Budget"
            || name == "School Budget"
            || name == "Project Budget"
        {
            m.new_sheet()?;
            m.rename_sheet(1, "Summary")?;
            heading(&mut m, 1, "Budget overview", &["Measure", "Amount"])?;
            m.set_user_input(1, 4, 1, "Total budget")?;
            m.set_user_input(1, 4, 2, &format!("=SUM('{}'!C4:C103)", name))?;
            m.set_user_input(1, 5, 1, "Actual spending")?;
            m.set_user_input(1, 5, 2, &format!("=SUM('{}'!D4:D103)", name))?;
            m.set_user_input(1, 6, 1, "Remaining")?;
            m.set_user_input(1, 6, 2, "=B4-B5")?;
        }
    }
    meta.settings = serde_json::json!({"layout":"Endless"});
    m.resume_evaluation();
    m.evaluate();
    Ok((m, meta))
}
pub fn col(mut n: i32) -> String {
    let mut result = String::new();
    while n > 0 {
        n -= 1;
        result.insert(0, (b'A' + (n % 26) as u8) as char);
        n /= 26;
    }
    result
}

fn literal(m: &mut UserModel, sheet: u32, row: i32, column: i32, text: &str) -> Result<(), String> {
    // Names, notes and references are data, even if they begin with =, + or @.
    m.set_user_input(sheet, row, column, &format!("'{text}"))
}
fn values(m: &mut UserModel, sheet: u32, row: i32, items: &[String], numbers: &[i32]) -> Result<(), String> {
    for (i, value) in items.iter().enumerate() {
        let c = i as i32 + 1;
        if numbers.contains(&c) { m.set_user_input(sheet, row, c, value)?; }
        else { literal(m, sheet, row, c, value)?; }
    }
    Ok(())
}
fn total(m: &mut UserModel, sheet: u32, row: i32, columns: &[i32], width: i32) -> Result<(), String> {
    literal(m, sheet, row, 1, "TOTAL")?;
    for &column in columns {
        let formula = if row == 4 { "=0".into() } else { format!("=SUM({}4:{}{})",col(column),col(column),row-1) };
        m.set_user_input(sheet, row, column, &formula)?;
        m.update_range_style(&Area { sheet, row: 4, column, width: 1, height: row-3 }, "num_fmt", "#,##0.00")?;
    }
    let area = Area { sheet, row, column: 1, width, height: 1 };
    m.update_range_style(&area, "font.b", "true")?;
    m.update_range_style(&area, "fill.color", "#E8DDF2")?;
    m.update_range_style(&area, "font.color", "#251332")?;
    Ok(())
}
fn clear_projection(m: &mut UserModel, sheet: u32) -> Result<(), String> {
    let dim = m.get_model().workbook.worksheets[sheet as usize].dimension();
    let area = Area { sheet, row: 1, column: 1, width: dim.max_column.max(20), height: dim.max_row.max(4) };
    m.range_clear_contents(&area)?;
    m.update_range_style(&area, "font.b", "false")?;
    m.update_range_style(&area, "fill.color", "#FFFFFF")?;
    m.update_range_style(&area, "font.color", "#000000")?;
    Ok(())
}
pub fn sync(m: &mut UserModel, meta: &Metadata) -> Result<(), String> {
    if meta.business.is_none() { return Ok(()); }
    m.pause_evaluation();
    crate::custom::expand(m)?;
    crate::custom::restore_stock(m,meta)?;
    let result = sync_inner(m, meta).and_then(|_|crate::custom::project(m,meta));
    m.resume_evaluation();
    m.evaluate();
    result
}
fn sync_inner(m: &mut UserModel, meta: &Metadata) -> Result<(), String> {
    let b = meta.business.as_ref().ok_or("Business settings missing")?;
    while m.get_worksheets_properties().len() < 16 { m.new_sheet()?; }
    for (sheet,name) in [(4,"Employer Tax & Contributions"),(12,"Employee Tax & Contributions"),(13,"Timesheets"),(14,"Fuel"),(15,"Vehicle Repairs")] {
        if m.get_model().workbook.worksheets[sheet as usize].name != name { m.rename_sheet(sheet,name)?; }
    }
    for s in 0..17 { if s != 8 { clear_projection(m, s)?; } }
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    for (category, sheet, title) in [("Income",1,"Income"),("Expense",2,"Expenses"),("Asset",6,"Assets"),("Refund",7,"Refunds")] {
        heading(m, sheet, title, &["Record ID","Period","Customer / recipient","Description","Amount","Paid","Outstanding","Due date","Status","Paid date","Tax","Deductible","Invoice number","Deductible base","Deduction treatment","Tax percent","Tax fixed","Deductible percent","Deductible fixed"])?;
        literal(m,sheet,2,1,"Live records. Totals cover all listed periods; paid dates come from payment history.")?;
        let mut row = 4;
        for r in meta.records.iter().filter(|r| r.category == category || (category == "Expense" && r.category == "Other payable")) {
            let bal = balance(r, &today)?;
            let (tax, deduction) = crate::live::transaction_tax(r)?;
            values(m,sheet,row,&[r.id.clone(),r.period.clone(),r.party.clone(),r.description.clone(),r.amount.clone(),bal.paid,bal.outstanding,r.due.clone(),bal.status,bal.paid_date,tax.to_string(),deduction.to_string(),r.invoice.clone(),r.calculation.deductible_base.clone(),r.calculation.deduction_target.clone(),r.calculation.tax_percent.clone(),r.calculation.tax_fixed.clone(),r.calculation.deductible_percent.clone(),r.calculation.deductible_fixed.clone()],&[5,6,7,11,12,16,17,18,19])?;
            row += 1;
        }
        total(m,sheet,row,&[5,6,7,11,12],12)?;
    }
    heading(m,3,"Live payroll", &["Employee","Structure","Gross","Employee tax","Employee contributions","Net salary","Employer contributions","Employer cost","Employer-paid tax","Employment status","Salary paid","Salary remaining","Paid date","Payment status","Days present","Ordinary hours","Deduction shortfall","Employee number","Work site"])?;
    literal(m,3,2,1,"Live attendance and deductions. Deduction shortfall means accrued earnings do not yet cover the deductions.")?;
    let mut row = 4;
    for e in &meta.employees {
        let p = payroll(e, &b.period)?;
        let key = format!("{}:payroll:{}",b.period,e.id);
        let paid = meta.records.iter().find(|r| r.managed_key == key).map(|r| balance(r,&today)).transpose()?;
        let days = e.days.iter().filter(|d| d.date.starts_with(&b.period) && d.status == "Present").count();
        let hours: rust_decimal::Decimal = e.days.iter().filter(|d| d.date.starts_with(&b.period)).map(|d| crate::finance::nonnegative(&d.hours)).collect::<Result<Vec<_>,_>>()?.into_iter().sum();
        let (amount,remaining,date,status) = paid.map(|v|(v.paid,v.outstanding,v.paid_date,v.status)).unwrap_or(("0".into(),p.net.clone(),String::new(),"Outstanding".into()));
        values(m,3,row,&[e.name.clone(),e.structure.clone(),p.gross,p.tax,p.employee_contributions,p.net,p.employer_contributions,p.employer_cost,p.employer_tax,if e.status.is_empty(){"Active".into()}else{e.status.clone()},amount,remaining,date,status,days.to_string(),hours.to_string(),p.deduction_shortfall,e.number.clone(),e.site.clone()],&[3,4,5,6,7,8,9,11,12,15,16,17])?;
        row+=1;
    }
    total(m,3,row,&[3,4,5,6,7,8,9,11,12,15,16,17],17)?;

    // Named identifiers remain text (including leading zeros and long numbers).
    let names: std::collections::BTreeSet<_> = meta.employees.iter().flat_map(|e|e.identifiers.iter().map(|f|f.name.clone())).collect();
    let contribution_names: std::collections::BTreeSet<_> = meta.employees.iter().flat_map(|e|e.contributions.iter().map(|c|c.name.clone())).collect();
    let mut col=20;
    for name in names { if !crate::custom::includes(meta,"Payroll","Employee identifiers"){continue;} literal(m,3,3,col,&name)?; for (i,e) in meta.employees.iter().enumerate(){ if let Some(f)=e.identifiers.iter().find(|f|f.name==name){literal(m,3,i as i32+4,col,&f.value)?;} } m.set_columns_width(3,col,col,220.0)?;col+=1; }
    for name in contribution_names { if !crate::custom::includes(meta,"Payroll","Individual contributions"){continue;} for employer in [false,true] { literal(m,3,3,col,&format!("{} · {}",name,if employer{"Employer"}else{"Employee"}))?;for(i,e)in meta.employees.iter().enumerate(){let p=payroll(e,&b.period)?;let amount=p.contributions.iter().filter(|c|c.name==name).map(|c|crate::finance::money(if employer{&c.employer}else{&c.employee})).collect::<Result<Vec<_>,_>>()?.into_iter().sum::<rust_decimal::Decimal>();m.set_user_input(3,i as i32+4,col,&amount.to_string())?;}total(m,3,meta.employees.len() as i32+4,&[col],col)?;m.set_columns_width(3,col,col,240.0)?;col+=1;}}
    heading(m,16,"Payslips",&["Employee","Employee number","Period","Item","Employee amount","Employer amount","Identifier value"])?;
    let mut sliprow=4;
    for e in &meta.employees { let p=payroll(e,&b.period)?;
        for f in &e.identifiers {if !crate::custom::includes(meta,"Payslips","Employee identifiers"){continue;}values(m,16,sliprow,&[e.name.clone(),e.number.clone(),b.period.clone(),f.name.clone(),String::new(),String::new(),f.value.clone()],&[])?;sliprow+=1;}
        let mut lines=vec![("Gross pay".to_string(),p.gross.clone(),"0".to_string()),("Tax".into(),p.tax.clone(),p.employer_tax.clone())];
        for c in &p.contributions {if !crate::custom::includes(meta,"Payslips","Individual contributions"){continue;}lines.push((c.name.clone(),c.employee.clone(),c.employer.clone()));}
        lines.push(("Total contributions".into(),p.employee_contributions.clone(),p.employer_contributions.clone()));
        lines.push(("Net salary / employer cost".into(),p.net.clone(),p.employer_cost.clone()));
        for(name,employee,employer)in lines {if !crate::custom::includes(meta,"Payslips",&name){continue;}values(m,16,sliprow,&[e.name.clone(),e.number.clone(),b.period.clone(),name,employee,employer,String::new()],&[5,6])?;sliprow+=1;}
        sliprow+=1;
    }
    let taxes = crate::live::tax_view(meta)?;
    for (sheet,title,employee) in [(4,"Employer taxes and contributions",false),(12,"Employee remittances",true)] {
        heading(m,sheet,title,&["Source / employee","Tax or contribution name","Type",if employee {"Employee amount"}else{"Employer amount"},"Tax credits","Income deductions"])?;
        m.set_columns_width(sheet,1,1,300.0)?;
        m.set_columns_width(sheet,2,2,260.0)?;
        m.set_columns_width(sheet,4,5,190.0)?;
        literal(m,sheet,2,1,if employee {"Amounts the employer remits on behalf of employees; these are withheld from employee pay."}else{"Employer's own amounts. Transaction deductions reduce employer tax only; unused deductions are shown separately."})?;
        row=4;
        for t in &taxes.rows {
            let amount = if employee { &t.employee } else { &t.employer };
            let deduction = if employee || t.kind == "Income deduction" { "0" } else { &t.deductible };
            let income_deduction = if !employee && t.kind == "Income deduction" { &t.deductible } else { "0" };
            // Keep configured contribution names visible even when this side owes zero.
            if t.kind != "Contribution" && crate::finance::money(amount)? == rust_decimal::Decimal::ZERO && crate::finance::money(deduction)? == rust_decimal::Decimal::ZERO && crate::finance::money(income_deduction)? == rust_decimal::Decimal::ZERO { continue; }
            values(m,sheet,row,&[t.source.clone(),t.name.clone(),t.kind.clone(),amount.clone(),deduction.into(),income_deduction.into()],&[4,5,6])?;
            row+=1;
        }
        total(m,sheet,row,&[4,5,6],6)?;
        row+=2;
        let metrics = if employee {
            vec![("Employee tax",&taxes.employee_tax),("Total employee contributions",&taxes.employee_contributions),("Total employee remittances",&taxes.employee_due)]
        } else {
            vec![("Income less refunds",&taxes.income),("Income deductions",&taxes.income_deductions),("Taxable income",&taxes.taxable_income),("Employer tax before deductions",&taxes.employer_tax),("Transaction deductions",&taxes.deductions),("Employer tax after deductions",&taxes.net_employer_tax),("Unused deductions",&taxes.unused_deductions),("Total employer contributions",&taxes.employer_contributions),("Total employer tax and contributions",&taxes.employer_due)]
        };
        for (label,amount) in metrics {
            literal(m,sheet,row,1,label)?; m.set_user_input(sheet,row,4,amount)?;
            m.update_range_style(&Area{sheet,row,column:4,width:1,height:1},"num_fmt","#,##0.00")?;
            m.update_range_style(&Area{sheet,row,column:1,width:5,height:1},"font.b","true")?;
            row+=1;
        }
    }

    heading(m,10,"Outstanding balances",&["Record ID","Source period","Category","Party","Amount","Paid","Remaining","Due date","Status","Paid date","Invoice number"])?;
    row=4;
    for r in &meta.records {
        let bal=balance(r,&today)?;
        if crate::finance::money(&bal.outstanding)? == rust_decimal::Decimal::ZERO { continue; }
        values(m,10,row,&[r.id.clone(),r.period.clone(),r.category.clone(),r.party.clone(),r.amount.clone(),bal.paid,bal.outstanding,r.due.clone(),bal.status,bal.paid_date,r.invoice.clone()],&[5,6,7])?;
        row+=1;
    }
    total(m,10,row,&[5,6,7],10)?;
    heading(m,11,"Payment history",&["Payment ID","Transaction ID","Date","Amount","Method","Reference","Reversal of","Notes","Invoice number"])?;
    row=4;
    for r in &meta.records { for p in &r.payments {
        values(m,11,row,&[p.id.clone(),r.id.clone(),p.date.clone(),p.amount.clone(),p.method.clone(),p.reference.clone(),p.reversal_of.clone().unwrap_or_default(),p.notes.clone(),r.invoice.clone()],&[4])?;
        row+=1;
    }}
    total(m,11,row,&[4],8)?;
    heading(m,13,"Timesheets",&["Employee","Date","Attendance","Ordinary hours","Overtime hours","Additional earnings","Fixed deduction","Notes","Employee number","Work site"])?;
    row=4;
    for e in &meta.employees {
        let mut days: Vec<_> = e.days.iter().filter(|d|d.date.starts_with(&b.period)).collect();
        days.sort_by_key(|d| &d.date);
        for d in days {
            values(m,13,row,&[e.name.clone(),d.date.clone(),d.status.clone(),d.hours.clone(),d.overtime.clone(),d.extra.clone(),d.deduction.clone(),d.note.clone(),e.number.clone(),e.site.clone()],&[4,5,6,7])?;
            row+=1;
        }
    }
    total(m,13,row,&[4,5,6,7],8)?;


    heading(m,14,"Fuel purchases",&["Record ID","Period","Petrol station","Vehicle registration","Employee","Price per litre","Litres","Total paid / invoiced","Tax percent","Tax fixed","Tax included","Deductible percent","Deductible fixed","Deductible base","Deduction treatment","Deductible amount","Invoice number","Paid","Outstanding","Paid date"])?;
    row=4;
    for r in meta.records.iter().filter(|r|r.category=="Fuel") {
        let v=&r.vehicle; let c=&r.calculation; let (tax,deduction)=crate::live::transaction_tax(r)?;let bal=balance(r,&today)?;
        values(m,14,row,&[r.id.clone(),r.period.clone(),r.party.clone(),v.registration.clone(),v.employee.clone(),v.price_per_litre.clone(),v.litres.clone(),r.amount.clone(),c.tax_percent.clone(),c.tax_fixed.clone(),tax.to_string(),c.deductible_percent.clone(),c.deductible_fixed.clone(),c.deductible_base.clone(),c.deduction_target.clone(),deduction.to_string(),r.invoice.clone(),bal.paid,bal.outstanding,bal.paid_date],&[6,7,8,9,10,11,12,13,16,18,19])?;row+=1;
    }
    total(m,14,row,&[7,8,10,11,13,16,18,19],20)?;
    for(c,label)in ["Discount percent","Discount fixed","Discount total","Start odometer","End odometer","Distance","Fuel economy","Volume unit","Distance unit","Economy unit","Description"].iter().enumerate(){literal(m,14,3,c as i32+21,label)?;}
    for(i,r)in meta.records.iter().filter(|r|r.category=="Fuel").enumerate(){let row=i as i32+4;let (distance,economy)=crate::custom::fuel_stats(r,&b.country)?;let u=crate::custom::units(&b.country,&r.vehicle.units);for(c,value)in [r.pricing.discount_percent.clone(),r.pricing.discount_fixed.clone(),crate::finance::percent_fixed(crate::finance::nonnegative(&r.vehicle.price_per_litre)?*crate::finance::nonnegative(&r.vehicle.litres)?,&r.pricing.discount_percent,&r.pricing.discount_fixed)?.to_string(),r.vehicle.start_odometer.clone(),r.vehicle.end_odometer.clone(),distance,economy,u.0.into(),u.1.into(),u.2.into(),r.description.clone()].iter().enumerate(){if c<7 {m.set_user_input(14,row,c as i32+21,value)?;}else{literal(m,14,row,c as i32+21,value)?;}}}
    literal(m,14,3,6,"Price per unit")?;literal(m,14,3,7,"Fuel quantity")?;
    for(c,label)in ["Gross amount","Discount percent","Discount fixed","Discount total"].iter().enumerate(){literal(m,1,3,c as i32+20,label)?;}
    for(i,r)in meta.records.iter().filter(|r|r.category=="Income").enumerate(){let gross=if r.pricing.enabled{r.pricing.gross.clone()}else{r.amount.clone()};for(c,value)in [gross.clone(),r.pricing.discount_percent.clone(),r.pricing.discount_fixed.clone(),(crate::finance::nonnegative(&gross)?-crate::finance::nonnegative(&r.amount)?).to_string()].iter().enumerate(){m.set_user_input(1,i as i32+4,c as i32+20,value)?;}}
    total(m,1,meta.records.iter().filter(|r|r.category=="Income").count() as i32+4,&[20,22,23],23)?;
    heading(m,15,"Vehicle repairs",&["Record ID","Period","Repair shop","Vehicle registration","Employee","Part name","Part number","Parts paid / invoiced","Labour paid / invoiced","Total paid / invoiced","Part tax percent","Part tax fixed","Part tax included","Labour tax percent","Labour tax fixed","Labour tax included","Part deductible percent","Part deductible fixed","Part deductible base","Part deduction treatment","Part deductible","Labour deductible percent","Labour deductible fixed","Labour deductible base","Labour deduction treatment","Labour deductible","Invoice number","Paid","Outstanding","Paid date","Description"])?;
    row=4;
    for r in meta.records.iter().filter(|r|r.category=="Vehicle repairs") {
        let v=&r.vehicle; let a=&v.part;let c=&v.labour;
        let pa=crate::live::component_tax(crate::finance::nonnegative(&v.part_amount)?,a)?;
        let la=crate::live::component_tax(crate::finance::nonnegative(&v.labour_amount)?,c)?;let bal=balance(r,&today)?;
        values(m,15,row,&[r.id.clone(),r.period.clone(),r.party.clone(),v.registration.clone(),v.employee.clone(),v.part_name.clone(),v.part_number.clone(),v.part_amount.clone(),v.labour_amount.clone(),r.amount.clone(),a.tax_percent.clone(),a.tax_fixed.clone(),pa.0.to_string(),c.tax_percent.clone(),c.tax_fixed.clone(),la.0.to_string(),a.deductible_percent.clone(),a.deductible_fixed.clone(),a.deductible_base.clone(),a.deduction_target.clone(),pa.1.to_string(),c.deductible_percent.clone(),c.deductible_fixed.clone(),c.deductible_base.clone(),c.deduction_target.clone(),la.1.to_string(),r.invoice.clone(),bal.paid,bal.outstanding,bal.paid_date,r.description.clone()],&[8,9,10,11,12,13,14,15,16,17,18,21,22,23,26,28,29])?;row+=1;
    }
    total(m,15,row,&[8,9,10,12,13,15,16,18,21,23,26,28,29],30)?;
    // Inventory remains editable; never rebuild its source rows from ledger metadata.
    heading(m,8,"Inventory",&["Product","Opening units","Received","Sold","Closing units","Unit cost","Stock value"])?;
    literal(m,8,2,1,"Total stock value")?;
    m.set_user_input(8,2,7,"=SUM(G4:G103)")?;
    total(m,8,105,&[2,3,4,5,7],7)?;
    // Row 104 is a spacer; total() includes it harmlessly, while top summary excludes it.
    let totals=summary(meta,&b.period,&b.period)?;
    for sheet in [0,5] {
        heading(m,sheet,if sheet==0 {&b.name}else{"Accrual profit and cash flow"},&["Current period measure","Amount"])?;
        literal(m,sheet,2,1,&format!("{} · {} · live calculations",b.period,b.currency))?;
        row=4;
        for key in ["revenue","expenses","profit","cash_received","cash_paid","cash_movement","receivables","payables"] {
            literal(m,sheet,row,1,&key.replace('_'," "))?;
            m.set_user_input(sheet,row,2,totals[key].as_str().unwrap_or("0"))?;
            row+=1;
        }
        for (label,amount) in [("Employer tax after deductions",&taxes.net_employer_tax),("Employer contributions",&taxes.employer_contributions),("Employer tax and contributions",&taxes.employer_due),("Employee remittances",&taxes.employee_due)] {
            literal(m,sheet,row,1,label)?;m.set_user_input(sheet,row,2,amount)?;row+=1;
        }
        let mut contribution_totals=std::collections::BTreeMap::<String,rust_decimal::Decimal>::new();
        for t in taxes.rows.iter().filter(|t|t.kind=="Contribution"){*contribution_totals.entry(t.name.clone()).or_default()+=crate::finance::money(&t.employer)?;}
        for(name,amount)in contribution_totals{literal(m,sheet,row,1,&format!("Employer contribution · {name}"))?;m.set_user_input(sheet,row,2,&amount.to_string())?;row+=1;}
        m.set_columns_width(sheet,1,1,300.0)?;
        m.update_range_style(&Area{sheet,row:4,column:2,width:1,height:row-4},"num_fmt","#,##0.00")?;
    }
    heading(m,9,"Business configuration",&["Setting","Value"])?;
    for (i,(key,value)) in [("Company ID",b.company_id.as_str()),("Business name",&b.name),("Country",&b.country),("Region",&b.region),("Currency",&b.currency),("Period",&b.period),("Accounting",&b.accounting),("Registration",&b.registration),("Tax registration",&b.tax_registration)].iter().enumerate() {
        literal(m,9,i as i32+4,1,key)?;literal(m,9,i as i32+4,2,value)?;
    }

    literal(m,9,15,1,"Employer tax ranges — current-period taxable income")?;
    for (col,title) in ["Tax name","Income above","Income cap","Percent","Fixed (blank = carry)","Mode"].iter().enumerate(){literal(m,9,16,col as i32+1,title)?;}
    row=17;
    for t in &meta.business_taxes {
        for band in &t.bands { values(m,9,row,&[t.name.clone(),band.lower.clone(),band.upper.clone(),band.percent.clone(),band.fixed.clone(),if t.automatic {"Automatic".into()}else{"Inactive / legacy".into()}],&[2,3,4,5])?;row+=1; }
    }
    m.set_columns_width(9,2,2,300.0)?;
    Ok(())
}

