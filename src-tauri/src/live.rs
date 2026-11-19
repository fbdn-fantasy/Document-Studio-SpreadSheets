use crate::finance::*;
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::BTreeSet;


pub fn component_tax(base: Decimal, c: &TransactionTax) -> Result<(Decimal, Decimal), String> {
    if !["", "Total", "Tax"].contains(&c.deductible_base.as_str()) || !["", "Income", "Tax"].contains(&c.deduction_target.as_str()) { return Err("Invalid deduction base or treatment".into()); }
    let tax = percent_fixed(base, &c.tax_percent, &c.tax_fixed)?;
    if c.purchase_tax && tax > base { return Err("Included purchase tax cannot exceed the amount paid".into()); }
    let deduction = percent_fixed(if c.deductible_base == "Tax" { tax } else { base }, &c.deductible_percent, &c.deductible_fixed)?;
    Ok((tax, deduction))
}
pub fn transaction_tax(r: &Record) -> Result<(Decimal, Decimal), String> {
    let base = nonnegative(&r.amount)?;
    if r.category == "Fuel" { nonnegative(&r.vehicle.price_per_litre)?; nonnegative(&r.vehicle.litres)?; }
    if r.category == "Vehicle repairs" {
        let part = nonnegative(&r.vehicle.part_amount)?;
        let labour = nonnegative(&r.vehicle.labour_amount)?;
        if part + labour != base { return Err("Repair total must equal parts plus labour".into()); }
        let a = component_tax(part, &r.vehicle.part)?;
        let b = component_tax(labour, &r.vehicle.labour)?;
        Ok((a.0+b.0,a.1+b.1))
    } else { component_tax(base, &r.calculation) }
}
pub fn transaction_deductions(r: &Record) -> Result<(Decimal,Decimal),String> {
    let mut income=Decimal::ZERO; let mut credit=Decimal::ZERO;
    let components = if r.category == "Vehicle repairs" { vec![(nonnegative(&r.vehicle.part_amount)?, &r.vehicle.part),(nonnegative(&r.vehicle.labour_amount)?, &r.vehicle.labour)] } else { vec![(nonnegative(&r.amount)?,&r.calculation)] };
    for (base,c) in components { let d=component_tax(base,c)?.1; if c.deduction_target == "Income" { income+=d; } else { credit+=d; } }
    Ok((income,credit))
}
pub fn transaction_liability(r: &Record) -> Result<Decimal,String> {
    // Fuel and repair taxes are already included in the supplier payment.
    if r.category == "Fuel" || r.category == "Vehicle repairs" || r.calculation.purchase_tax { Ok(Decimal::ZERO) } else { Ok(transaction_tax(r)?.0) }
}
pub fn income_base(m: &Metadata) -> Result<(Decimal,Decimal,Decimal),String> {
    let Some(b)=&m.business else { return Ok((Decimal::ZERO,Decimal::ZERO,Decimal::ZERO)) };
    let mut income=Decimal::ZERO; let mut deductions=Decimal::ZERO;
    for r in m.records.iter().filter(|r|r.period==b.period && r.managed_key.is_empty() && !imported_transaction(m,r)) {
        if r.category=="Income" { income+=nonnegative(&r.amount)?; }
        if r.category=="Refund" { income-=nonnegative(&r.amount)?; }
        deductions+=transaction_deductions(r)?.0;
    }
    income=income.max(Decimal::ZERO);
    for d in &m.deductions { if d.target=="Income" { deductions+=percent_fixed(income,&d.percent,&d.fixed)?; } }
    Ok((income,deductions,(income-deductions).max(Decimal::ZERO)))
}
pub fn rule_tax(t: &TaxRule, taxable: Decimal) -> Result<Decimal,String> {
    if !t.automatic { return percent_fixed(nonnegative(&t.base)?,&t.percent,&t.fixed); }
    if t.bands.is_empty() { return Err("Add at least one income range".into()); }
    let mut boundary=Decimal::ZERO; let mut carried=Decimal::ZERO; let mut result=None;
    for (i,b) in t.bands.iter().enumerate() {
        let lower=nonnegative(&b.lower)?;
        if lower!=boundary { return Err("Income ranges must start at zero and meet without gaps or overlaps".into()); }
        let upper=if b.upper.trim().is_empty() { None } else { Some(nonnegative(&b.upper)?) };
        if upper.is_some_and(|u|u<=lower) || (upper.is_none() && i+1!=t.bands.len()) { return Err("Each range needs a higher cap; only the final range may be unlimited".into()); }
        let fixed=if b.fixed.trim().is_empty() { carried } else { nonnegative(&b.fixed)? };
        percent_fixed(Decimal::ZERO,&b.percent,"0")?;
        if result.is_none() && taxable>=lower && upper.is_none_or(|u|taxable<=u) {
            result=Some(fixed+percent_fixed(taxable-lower,&b.percent,"0")?);
        }
        if let Some(u)=upper { carried=fixed+percent_fixed(u-lower,&b.percent,"0")?; boundary=u; }
        else { boundary=lower; }
    }
    if !t.bands.last().unwrap().upper.trim().is_empty() { return Err("Leave the final income cap blank for income above the last threshold".into()); }
    Ok(result.unwrap_or(Decimal::ZERO))
}
pub fn employee_source(e: &Employee) -> String { if e.number.is_empty(){e.name.clone()}else{format!("{} · {}",e.number,e.name)} }
#[derive(Serialize)]
pub struct TaxLine {
    pub name: String,
    pub source: String,
    pub kind: String,
    pub employee: String,
    pub employer: String,
    pub deductible: String,
}
#[derive(Serialize, Default)]
pub struct TaxView {
    pub rows: Vec<TaxLine>,
    pub employer_tax: String,
    pub deductions: String,
    pub net_employer_tax: String,
    pub unused_deductions: String,
    pub employer_contributions: String,
    pub employee_tax: String,
    pub employee_contributions: String,
    pub employer_due: String,
    pub employee_due: String,
    pub income: String,
    pub income_deductions: String,
    pub taxable_income: String,
}
pub fn tax_view(m: &Metadata) -> Result<TaxView, String> {
    let Some(b) = &m.business else { return Ok(TaxView::default()) };
    let mut out = TaxView::default();
    let (income,income_deductions,taxable)=income_base(m)?;
    out.income=income.to_string(); out.income_deductions=income_deductions.to_string(); out.taxable_income=taxable.to_string();
    let (mut et, mut ec, mut rt, mut rc, mut deductions) = (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO, Decimal::ZERO, Decimal::ZERO);
    let mut imported_tax = Decimal::ZERO;
    let mut ids = BTreeSet::new();
    for t in &m.business_taxes {
        if t.id.is_empty() || !ids.insert(&t.id) || t.name.trim().is_empty() { return Err("Each tax needs a unique ID and a name".into()); }
        let amount = rule_tax(t,taxable)?;
        rt += amount;
        out.rows.push(TaxLine { name: t.name.clone(), source: b.name.clone(), kind: "Tax".into(), employee: "0".into(), employer: amount.to_string(), deductible: "0".into() });
    }
    for r in m.records.iter().filter(|r| r.period == b.period && r.managed_key.is_empty()) {
        if imported_transaction(m, r) { continue; }
        let tax = transaction_liability(r)?;
        let (income_deduction, deductible) = transaction_deductions(r)?;
        if income_deduction > Decimal::ZERO { out.rows.push(TaxLine { name:r.description.clone(),source:r.party.clone(),kind:"Income deduction".into(),employee:"0".into(),employer:"0".into(),deductible:income_deduction.to_string() }); }
        // Preserve manually entered V1 tax records as separate liabilities.
        let legacy = if r.category == "Tax" { money(&r.amount)? } else { Decimal::ZERO };
        rt += tax + legacy;
        deductions += deductible;
        if tax + legacy > Decimal::ZERO || deductible > Decimal::ZERO {
            out.rows.push(TaxLine { name: r.description.clone(), source: r.party.clone(), kind: "Transaction tax / claim".into(), employee: "0".into(), employer: (tax + legacy).to_string(), deductible: deductible.to_string() });
        }
        if ["Employer contribution", "Employee withholding", "Payroll withholding"].contains(&r.category.as_str()) {
            let amount = money(&r.amount)?;
            let employer = r.category == "Employer contribution";
            let tax = !employer && r.description.to_lowercase().starts_with("tax");
            if employer { rc += amount; } else if tax { et += amount; } else { ec += amount; }
            out.rows.push(TaxLine { name: r.description.clone(), source: r.party.clone(), kind: if tax { "Tax".into() } else { "Contribution".into() }, employee: if employer { "0".into() } else { amount.to_string() }, employer: if employer { amount.to_string() } else { "0".into() }, deductible: "0".into() });
        }
    }
    for e in &m.employees {
        let p = payroll(e, &b.period)?;
        et += money(&p.tax)?;
        rt += money(&p.employer_tax)?;
        out.rows.push(TaxLine { name: "Payroll tax".into(), source: employee_source(e), kind: "Tax".into(), employee: p.tax, employer: p.employer_tax, deductible: "0".into() });
        for c in p.contributions {
            ec += money(&c.employee)?; rc += money(&c.employer)?;
            out.rows.push(TaxLine { name: c.name, source: employee_source(e), kind: "Contribution".into(), employee: c.employee, employer: c.employer, deductible: "0".into() });
        }
    }
    for c in &m.business_contributions {
        let a = contribution(c, nonnegative(&c.base)?)?;
        if money(&a.employee)? > Decimal::ZERO { return Err("Assign employee contributions to an employee in Payroll so their net salary is updated".into()); }
        rc += money(&a.employer)?;
        out.rows.push(TaxLine { name: c.name.clone(), source: b.name.clone(), kind: "Contribution".into(), employee: "0".into(), employer: a.employer, deductible: "0".into() });
    }

    let mut deduction_ids=BTreeSet::new();
    for d in &m.deductions {
        if d.id.is_empty() || !deduction_ids.insert(&d.id) || d.name.trim().is_empty() || !["Income","Tax"].contains(&d.target.as_str()) { return Err("Each deduction needs a unique ID, description and treatment".into()); }
        let amount=percent_fixed(income,&d.percent,&d.fixed)?;
        if d.target=="Tax" { deductions+=amount; }
        out.rows.push(TaxLine{name:d.name.clone(),source:b.name.clone(),kind:if d.target=="Income"{"Income deduction".into()}else{"Tax credit".into()},employee:"0".into(),employer:"0".into(),deductible:amount.to_string()});
    }
    let mut extra_ids=BTreeSet::new();
    for x in &m.employee_extras {
        if x.id.is_empty() || !extra_ids.insert(&x.id) || x.employee.trim().is_empty() || x.number.trim().is_empty() || x.name.trim().is_empty() || !["Tax","Contribution"].contains(&x.kind.as_str()) { return Err("An additional employee item needs employee number, name, description and type".into()); }
        let amount=percent_fixed(nonnegative(&x.base)?,&x.percent,&x.fixed)?;
        if x.kind=="Tax" { et+=amount; } else { ec+=amount; }
        out.rows.push(TaxLine{name:x.name.clone(),source:format!("{} · {}",x.number,x.employee),kind:x.kind.clone(),employee:amount.to_string(),employer:"0".into(),deductible:"0".into()});
    }
    // Consolidated months retain their source's final obligations and applied deductions.
    for r in m.records.iter().filter(|r| r.period == b.period && !r.managed_key.is_empty() && !r.source_workbook.is_empty() && r.source_workbook != m.workbook_id) {
        let amount = money(&r.amount)?;
        let (employee, employer, kind) = match r.category.as_str() {
            "Tax" | "Employer tax" => { imported_tax += amount; (Decimal::ZERO,amount,"Imported tax after source deductions") },
            "Employer contribution" => { rc += amount; (Decimal::ZERO,amount,"Contribution") },
            "Payroll withholding" if (r.managed_key.contains(":employee-tax:") || (r.managed_key.contains(":employee-extra:") && r.description.starts_with("Tax:"))) => { et += amount; (amount,Decimal::ZERO,"Tax") },
            "Payroll withholding" => { ec += amount; (amount,Decimal::ZERO,"Contribution") },
            _ => continue,
        };
        out.rows.push(TaxLine { name:r.description.clone(),source:r.party.clone(),kind:kind.into(),employee:employee.to_string(),employer:employer.to_string(),deductible:"0".into() });
    }
    let net = (rt - deductions).max(Decimal::ZERO) + imported_tax;
    out.employer_tax = (rt + imported_tax).to_string(); out.deductions = deductions.to_string();
    out.net_employer_tax = net.to_string(); out.unused_deductions = (deductions-rt).max(Decimal::ZERO).to_string();
    out.employer_contributions = rc.to_string(); out.employee_tax = et.to_string(); out.employee_contributions = ec.to_string();
    out.employer_due = (net + rc).to_string(); out.employee_due = (et + ec).to_string();
    Ok(out)
}
fn imported_transaction(m: &Metadata, r: &Record) -> bool {
    !r.source_workbook.is_empty() && r.source_workbook != m.workbook_id && m.records.iter().any(|t| t.source_workbook == r.source_workbook && t.managed_key == format!("{}:transaction-tax:{}",r.period,r.id))
}

// Rebuild obligations after every accepted edit. Stable keys retain all payment events.
pub fn sync_records(m: &mut Metadata) -> Result<(), String> {
    if m.schema>4 { return Err("Unsupported workbook metadata version".into()); }
    m.schema = 4;
    let Some(b) = m.business.clone() else { return Ok(()) };
    if m.closed_periods.contains(&b.period) { return Ok(()) }
    for r in m.records.iter_mut().filter(|r|r.period==b.period && r.managed_key.is_empty() && r.pricing.enabled) {
        r.amount=calculated_amount(r)?.to_string();
        if r.category=="Fuel" { fuel_distance(r)?; settle_fuel(r)?; }
    }
    // Upgrade V1 standalone taxes into editable named rules without losing payments.
    for r in m.records.iter_mut().filter(|r| r.period == b.period && r.category == "Tax" && r.managed_key.is_empty()) {
        let id = format!("legacy:{}",r.id);
        if !m.business_taxes.iter().any(|t| t.id == id) {
            m.business_taxes.push(TaxRule { id: id.clone(), name: r.description.clone(), base: "0".into(), percent: "0".into(), fixed: r.amount.clone(), ..Default::default() });
        }
        r.managed_key = format!("{}:employer-rule:{id}", b.period);
    }
    let taxes = tax_view(m)?;
    let taxable=money(&taxes.taxable_income)?;
    let mut wanted: Vec<(String,String,String,String,String)> = vec![];
    for e in &m.employees {
        let p = payroll(e, &b.period)?;
        wanted.push((format!("payroll:{}",e.id), "Payroll".into(), employee_source(e), format!("Payroll {}",e.id), p.net));
        wanted.push((format!("employee-tax:{}",e.id), "Payroll withholding".into(), employee_source(e), format!("Tax withheld {}",e.id), p.tax));
        // Aggregate payment obligations; the detail sheets retain each named contribution.
        wanted.push((format!("employee-contributions:{}",e.id), "Payroll withholding".into(), employee_source(e), format!("Employee contributions {}",e.id), p.employee_contributions));
        wanted.push((format!("employer-contributions:{}",e.id), "Employer contribution".into(), employee_source(e), format!("Employer contributions {}",e.id), p.employer_contributions));
    }
    for x in &m.employee_extras {
        wanted.push((format!("employee-extra:{}",x.id),"Payroll withholding".into(),format!("{} · {}",x.number,x.employee),format!("{}: {}",x.kind,x.name),percent_fixed(nonnegative(&x.base)?,&x.percent,&x.fixed)?.to_string()));
    }
    let mut credit = money(&taxes.deductions)?;
    let mut assessments = vec![];
    for t in &m.business_taxes {
        assessments.push((format!("employer-rule:{}",t.id), b.name.clone(), t.name.clone(), rule_tax(t,taxable)?));
    }
    for r in m.records.iter().filter(|r| r.period == b.period && r.managed_key.is_empty() && ["Income", "Expense", "Fuel", "Vehicle repairs", "Asset", "Refund", "Other payable"].contains(&r.category.as_str()) && !imported_transaction(m,r)) {
        assessments.push((format!("transaction-tax:{}",r.id), r.party.clone(), format!("Tax: {}",r.description), transaction_liability(r)?));
    }
    for e in &m.employees {
        assessments.push((format!("employer-payroll-tax:{}",e.id),employee_source(e),"Employer-paid payroll tax".into(),money(&payroll(e,&b.period)?.employer_tax)?));
    }
    for (key, party, description, gross) in assessments {
        let applied = credit.min(gross); credit -= applied;
        wanted.push((key, "Employer tax".into(), party, description, (gross-applied).to_string()));
    }
    let company_contributions: Decimal = m.business_contributions.iter().map(|c| contribution(c, nonnegative(&c.base)?).and_then(|a| money(&a.employer))).collect::<Result<Vec<_>,_>>()?.into_iter().sum();
    wanted.push(("business-contributions".into(), "Employer contribution".into(), b.name.clone(), "Business contributions".into(), company_contributions.to_string()));
    let mut keys = BTreeSet::new();
    for (key, category, party, description, amount) in wanted {
        let key = format!("{}:{key}", b.period);
        keys.insert(key.clone());
        let existing = m.records.iter_mut().find(|r| r.period == b.period && (r.source_workbook == m.workbook_id || r.source_workbook.is_empty()) && (r.managed_key == key || (r.managed_key.is_empty() && r.category == category && r.description == description)));
        if let Some(r) = existing {
            if r.source_workbook.is_empty() { r.source_workbook = m.workbook_id.clone(); }
            if r.amount != amount || r.party != party { r.revision += 1; }
            r.managed_key = key; r.amount = amount; r.party = party; r.description = description;
            balance(r, "").map_err(|_| "The revised obligation is below payments already recorded. Mark it unpaid (or reverse the relevant payments), then edit and record the corrected payment.".to_string())?;
        } else {
            m.records.push(Record { id: uuid::Uuid::new_v4().to_string(), revision: 1, period: b.period.clone(), category, party, description, amount, source_workbook: m.workbook_id.clone(), managed_key: key, ..Default::default() });
        }
    }
    for r in m.records.iter_mut().filter(|r| r.period == b.period && r.source_workbook == m.workbook_id && !r.managed_key.is_empty() && !keys.contains(&r.managed_key)) {
        if money(&balance(r, "")?.paid)? > Decimal::ZERO { return Err("Reverse the related payments before deleting this item. For employees, Resigned or Fired preserves this month's pay and history.".into()); }
        r.amount = "0".into(); r.revision += 1;
    }
    m.records.retain(|r| r.managed_key.is_empty() || r.source_workbook != m.workbook_id || r.period != b.period || keys.contains(&r.managed_key) || !r.payments.is_empty());
    Ok(())
}

pub fn rollover(source: &Metadata, business: Business) -> Result<Metadata, String> {
    if source.settings["custom_template"].is_object() && !(source.settings["custom_template"]["sheets"]["Income"]["enabled"]==true && source.settings["custom_template"]["sheets"]["Payroll"]["enabled"]==true){return Err("Rollover requires Income and Payroll worksheets".into());}
    let old = source.business.as_ref().ok_or("Source needs business settings")?;
    if business.period <= old.period { return Err("Choose a month after the current month".into()); }
    let mut next = Metadata { template: source.template.clone(), business: Some(business), business_taxes: source.business_taxes.clone(), business_contributions: source.business_contributions.clone(), ..Default::default() };
    next.settings=source.settings.clone();
    next.employees = source.employees.iter().filter(|e| e.status.is_empty() || e.status == "Active").cloned().map(|mut e| { e.days.clear(); e }).collect();
    next = merge(&next, source, true)?;
    sync_records(&mut next)?;
    validate(&next)?;
    Ok(next)
}

pub fn settle_fuel(r: &mut Record) -> Result<(),String> {
    let total=money(&r.amount)?;
    let paid: Decimal=r.payments.iter().map(|p|money(&p.amount)).collect::<Result<Vec<_>,_>>()?.into_iter().sum();
    if paid==total { return Ok(()); }
    let today=chrono::Local::now().format("%Y-%m-%d").to_string();
    let reversed:BTreeSet<String>=r.payments.iter().filter_map(|p|p.reversal_of.clone()).collect();
    let active:Vec<_>=r.payments.iter().filter(|p|p.reversal_of.is_none()&&!reversed.contains(&p.id)).cloned().collect();
    for p in active { r.payments.push(Payment{id:uuid::Uuid::new_v4().to_string(),date:today.clone(),amount:(-money(&p.amount)?).to_string(),reference:p.reference,method:p.method,reversal_of:Some(p.id),notes:"Fuel total adjustment".into()}); }
    if total>Decimal::ZERO { r.payments.push(Payment{id:uuid::Uuid::new_v4().to_string(),date:today,amount:total.to_string(),reference:r.invoice.clone(),method:"Fuel purchase".into(),reversal_of:None,notes:"Automatically paid fuel".into()}); }
    Ok(())
}

