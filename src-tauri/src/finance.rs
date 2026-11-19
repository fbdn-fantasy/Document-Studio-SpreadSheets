use chrono::{Datelike, NaiveDate};
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};
use uuid::Uuid;
use crate::live::{tax_view, transaction_tax};

pub fn money(s: &str) -> Result<Decimal, String> {
    let d = Decimal::from_str(s).map_err(|_| "Enter a valid decimal amount".to_string())?;
    if d.scale() > 2 {
        return Err("Money may have at most two decimal places".into());
    }
    Ok(d)
}
fn rounded(d: Decimal) -> Decimal {
    d.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}
fn zero() -> String {
    "0.00".into()
}
fn hundred() -> String { "100".into() }
fn active() -> String { "Active".into() }
fn employee_payer() -> String { "Employee".into() }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Payment {
    pub id: String,
    pub date: String,
    pub amount: String,
    pub reference: String,
    pub method: String,
    #[serde(default)]
    pub reversal_of: Option<String>,
    #[serde(default)]
    pub notes: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Record {
    #[serde(default)]
    pub pricing: Pricing,
    #[serde(default)]
    pub invoice: String,
    #[serde(default)]
    pub vehicle: Vehicle,
    pub id: String,
    pub revision: u32,
    pub period: String,
    pub category: String,
    pub party: String,
    pub description: String,
    pub amount: String,
    pub due: String,
    #[serde(default)]
    pub payments: Vec<Payment>,
    #[serde(default)]
    pub disputed: bool,
    #[serde(default)]
    pub source_workbook: String,
    #[serde(default)]
    pub calculation: TransactionTax,
    #[serde(default)]
    pub managed_key: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct TransactionTax {
    pub tax_percent: String,
    pub tax_fixed: String,
    pub deductible_percent: String,
    pub deductible_fixed: String,
    pub deductible_base: String,
    pub deduction_target: String,
    pub purchase_tax: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TaxRule {
    #[serde(default)]
    pub automatic: bool,
    #[serde(default)]
    pub bands: Vec<TaxBand>,
    pub id: String,
    pub name: String,
    pub base: String,
    pub percent: String,
    pub fixed: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Business {
    pub company_id: String,
    pub name: String,
    pub country: String,
    pub region: String,
    pub currency: String,
    pub kind: String,
    pub period: String,
    pub year_start: u32,
    pub accounting: String,
    #[serde(default)]
    pub registration: String,
    #[serde(default)]
    pub tax_registration: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Day {
    pub date: String,
    pub status: String,
    #[serde(default = "zero")]
    pub hours: String,
    #[serde(default = "zero")]
    pub overtime: String,
    #[serde(default = "zero")]
    pub extra: String,
    #[serde(default = "zero")]
    pub deduction: String,
    #[serde(default)]
    pub note: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Contribution {
    pub name: String,
    pub employee_percent: String,
    pub employer_percent: String,
    pub employee_fixed: String,
    pub employer_fixed: String,
    pub cap: Option<String>,
    #[serde(default = "hundred")]
    pub total_percent: String,
    #[serde(default = "zero")]
    pub total_fixed: String,
    #[serde(default = "zero")]
    pub base: String,
    #[serde(default)]
    pub split_pool: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Employee {
    #[serde(default)]
    pub identifiers: Vec<Identifier>,
    #[serde(default)]
    pub number: String,
    #[serde(default)]
    pub site: String,
    pub id: String,
    pub name: String,
    pub role: String,
    pub structure: String,
    pub rate: String,
    pub standard_hours: String,
    pub overtime_multiplier: String,
    #[serde(default)]
    pub days: Vec<Day>,
    #[serde(default)]
    pub contributions: Vec<Contribution>,
    #[serde(default = "zero")]
    pub manual_tax: String,
    #[serde(default = "zero")]
    pub tax_percent: String,
    #[serde(default = "employee_payer")]
    pub tax_payer: String,
    #[serde(default = "active")]
    pub status: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub schema: u32,
    pub workbook_id: String,
    pub template: String,
    pub business: Option<Business>,
    pub records: Vec<Record>,
    pub employees: Vec<Employee>,
    pub closed_periods: BTreeSet<String>,
    pub lineage: BTreeMap<String, u32>,
    #[serde(default)]
    pub audit: Vec<String>,
    #[serde(default)]
    pub settings: serde_json::Value,
    #[serde(default)]
    pub business_taxes: Vec<TaxRule>,
    #[serde(default)]
    pub business_contributions: Vec<Contribution>,
    #[serde(default)]
    pub deductions: Vec<ManualDeduction>,
    #[serde(default)]
    pub employee_extras: Vec<EmployeeExtra>,
}
impl Default for Metadata {
    fn default() -> Self {
        Self {
            schema: 4,
            workbook_id: Uuid::new_v4().to_string(),
            template: "Blank workbook".into(),
            business: None,
            records: vec![],
            employees: vec![],
            closed_periods: BTreeSet::new(),
            lineage: BTreeMap::new(),
            audit: vec![],
            settings: serde_json::json!({}),
            business_taxes: vec![],
            business_contributions: vec![],
            deductions: vec![],
            employee_extras: vec![],
        }
    }
}
#[derive(Serialize)]
pub struct Balance {
    pub id: String,
    pub paid: String,
    pub outstanding: String,
    pub status: String,
    pub paid_date: String,
}
pub fn balance(r: &Record, today: &str) -> Result<Balance, String> {
    let total = money(&r.amount)?;
    if total < Decimal::ZERO {
        return Err("Transaction amount cannot be negative".into());
    }
    let mut ids = BTreeSet::new();
    let mut reversed = BTreeSet::new();
    let mut paid = Decimal::ZERO;
    for p in &r.payments {
        if !ids.insert(&p.id) {
            return Err("Duplicate payment identifier".into());
        }
        NaiveDate::parse_from_str(&p.date, "%Y-%m-%d").map_err(|_| "Invalid payment date")?;
        let amount = money(&p.amount)?;
        if let Some(original) = &p.reversal_of {
            let old = r
                .payments
                .iter()
                .find(|x| x.id == *original && x.reversal_of.is_none())
                .ok_or("Original payment not found")?;
            if amount != -money(&old.amount)? || !reversed.insert(original) {
                return Err("Invalid or duplicate reversal".into());
            }
        } else if amount <= Decimal::ZERO {
            return Err("Payment must be positive".into());
        }
        paid += amount;
    }
    if paid < Decimal::ZERO || paid > total {
        return Err("Payment exceeds outstanding balance".into());
    }
    let remaining = total - paid;
    let status = if r.disputed {
        "Disputed"
    } else if remaining == Decimal::ZERO {
        "Settled"
    } else if !r.due.is_empty() && r.due.as_str() < today {
        "Overdue"
    } else if paid > Decimal::ZERO {
        "Partially settled"
    } else {
        "Outstanding"
    };
    Ok(Balance {
        id: r.id.clone(),
        paid: paid.to_string(),
        outstanding: remaining.to_string(),
        status: status.into(),
        paid_date: if remaining == Decimal::ZERO && total > Decimal::ZERO { r.payments.iter().filter(|p| p.reversal_of.is_none() && !reversed.contains(&p.id)).map(|p| p.date.clone()).max().unwrap_or_default() } else { String::new() },
    })
}
#[derive(Debug, Serialize)]
pub struct Payroll {
    pub gross: String,
    pub tax: String,
    pub employer_tax: String,
    pub employee_contributions: String,
    pub employer_contributions: String,
    pub net: String,
    pub deduction_shortfall: String,
    pub employer_cost: String,
    pub explanation: Vec<String>,
    pub contributions: Vec<ContributionAmount>,
}
#[derive(Debug, Serialize)]
pub struct ContributionAmount {
    pub name: String,
    pub total: String,
    pub employee: String,
    pub employer: String,
}
pub fn nonnegative(s: &str) -> Result<Decimal, String> {
    let d = if s.is_empty() { Decimal::ZERO } else { Decimal::from_str(s).map_err(|_| "Enter a valid number")? };
    if d < Decimal::ZERO { return Err("Amounts and percentages must be non-negative".into()); }
    Ok(d)
}
pub fn percent_fixed(base: Decimal, percent: &str, fixed: &str) -> Result<Decimal, String> {
    let p = nonnegative(percent)?;
    let f = if fixed.is_empty() { Decimal::ZERO } else { money(fixed)? };
    if f < Decimal::ZERO || base < Decimal::ZERO { return Err("Amounts must be non-negative".into()); }
    Ok(rounded(base * p / Decimal::from(100) + f))
}
pub fn contribution(c: &Contribution, gross: Decimal) -> Result<ContributionAmount, String> {
    if c.name.trim().is_empty() { return Err("Give each contribution a name".into()); }
    let base = if let Some(cap) = &c.cap { gross.min(nonnegative(cap)?) } else { gross };
    let total = if c.split_pool { percent_fixed(base, &c.total_percent, &c.total_fixed)? } else { base };
    let ep = nonnegative(&c.employee_percent)?;
    let er = nonnegative(&c.employer_percent)?;
    // V1 used percentages directly against gross; total_percent defaults to 100 on migration.
    // A new pooled contribution must be fully assigned whenever a pool is entered.
    if c.split_pool && total > Decimal::ZERO && ep + er != Decimal::from(100) {
        return Err("Employee and employer percentages must total 100% to allocate the contribution pool".into());
    }
    let employee = percent_fixed(total, &c.employee_percent, &c.employee_fixed)?;
    let employer = if c.split_pool && total > Decimal::ZERO {
        // Allocate the rounding remainder so a shared cent is never charged twice.
        total - percent_fixed(total, &c.employee_percent, "0")? + percent_fixed(Decimal::ZERO, "0", &c.employer_fixed)?
    } else { percent_fixed(total, &c.employer_percent, &c.employer_fixed)? };
    Ok(ContributionAmount { name: c.name.clone(), total: (employee + employer).to_string(), employee: employee.to_string(), employer: employer.to_string() })
}
pub fn payroll(e: &Employee, period: &str) -> Result<Payroll, String> {
    let rate = money(&e.rate)?;
    let standard = Decimal::from_str(&e.standard_hours).map_err(|_| "Invalid standard hours")?;
    let multiplier = Decimal::from_str(&e.overtime_multiplier).map_err(|_| "Invalid multiplier")?;
    if rate < Decimal::ZERO || standard <= Decimal::ZERO || multiplier < Decimal::ZERO {
        return Err("Invalid pay settings".into());
    }
    let mut gross = if e.structure == "Monthly" {
        rate
    } else {
        Decimal::ZERO
    };
    let mut deductions = Decimal::ZERO;
    let mut seen = BTreeSet::new();
    let hourly = match e.structure.as_str() {
        "Hourly" => rate,
        "Daily" => rate / standard,
        "Monthly" => Decimal::ZERO,
        _ => return Err("Unknown pay structure".into()),
    };
    let mut explanation = vec![format!(
        "{} rate: {}. Contractual inputs; statutory compliance requires review.",
        e.structure, rate
    )];
    for d in &e.days {
        NaiveDate::parse_from_str(&d.date, "%Y-%m-%d").map_err(|_| "Invalid attendance date")?;
        if !seen.insert(&d.date) {
            return Err("Duplicate attendance date".into());
        }
        if !d.date.starts_with(period) {
            continue;
        }
        let hours = Decimal::from_str(&d.hours).map_err(|_| "Invalid hours")?;
        let overtime = Decimal::from_str(&d.overtime).map_err(|_| "Invalid overtime")?;
        if hours < Decimal::ZERO
            || hours > Decimal::from(24)
            || overtime < Decimal::ZERO
            || hours + overtime > Decimal::from(24)
        {
            return Err("Hours must be between 0 and 24 per day".into());
        }
        if matches!(
            d.status.as_str(),
            "Present" | "Paid leave" | "Public holiday" | "Sick leave"
        ) && e.structure != "Monthly"
        {
            gross += hours * hourly;
        }
        if e.structure == "Monthly" && overtime > Decimal::ZERO {
            return Err("For monthly employees, enter agreed overtime as additional earnings; no hourly divisor is assumed".into());
        }
        let extra = money(&d.extra)?;
        let deduction = money(&d.deduction)?;
        if extra < Decimal::ZERO || deduction < Decimal::ZERO {
            return Err("Additional earnings and deductions must be non-negative".into());
        }
        gross += overtime * hourly * multiplier + extra;
        deductions += deduction;
    }
    gross = rounded(gross);
    let mut employee = Decimal::ZERO;
    let mut employer = Decimal::ZERO;
    let mut contributions = vec![];
    for c in &e.contributions {
        let result = contribution(c, gross)?;
        employee += money(&result.employee)?;
        employer += money(&result.employer)?;
        explanation.push(format!("{}: employee {}, employer {}", result.name, result.employee, result.employer));
        contributions.push(result);
    }
    let calculated_tax = percent_fixed(gross, &e.tax_percent, &e.manual_tax)?;
    let (tax, employer_tax) = if e.tax_payer == "Employer" { (Decimal::ZERO, calculated_tax) } else { (calculated_tax, Decimal::ZERO) };
    let net_before_floor = rounded(gross - tax - employee - deductions);
    let net = net_before_floor.max(Decimal::ZERO);
    let deduction_shortfall = (-net_before_floor).max(Decimal::ZERO);
    if deduction_shortfall > Decimal::ZERO {
        explanation.push(format!("Deduction shortfall: {} is not covered by accrued earnings. Net salary is zero until further earnings cover the configured deductions.", deduction_shortfall));
    }
    explanation.push("Tax is a manually entered amount, not verified automated PAYE.".into());
    Ok(Payroll {
        gross: gross.to_string(),
        tax: tax.to_string(),
        employer_tax: employer_tax.to_string(),
        employee_contributions: employee.to_string(),
        employer_contributions: employer.to_string(),
        net: net.to_string(),
        deduction_shortfall: deduction_shortfall.to_string(),
        employer_cost: (gross + employer + employer_tax).to_string(),
        contributions,
        explanation,
    })
}
pub fn validate(m: &Metadata) -> Result<(), String> {
    if m.schema != 1 && m.schema != 2 && m.schema != 3 && m.schema != 4 {
        return Err("Unsupported workbook metadata version".into());
    }
    let mut ids = BTreeSet::new();
    let mut payments = BTreeSet::new();
    if let Some(b) = &m.business {
        if b.currency.len() != 3 || b.company_id.is_empty() || !(1..=12).contains(&b.year_start) {
            return Err("Invalid business identity/currency/year".into());
        }
        valid_period(&b.period)?;
    }
    for r in &m.records {
        if r.id.is_empty() || !ids.insert(&r.id) {
            return Err("Duplicate transaction identifier".into());
        }
        valid_period(&r.period)?;
        if !r.due.is_empty() {
            NaiveDate::parse_from_str(&r.due, "%Y-%m-%d").map_err(|_| "Invalid due date")?;
        }
        balance(r, "")?;
        transaction_tax(r)?;
        for p in &r.payments {
            if !payments.insert(&p.id) {
                return Err("Payment ID repeated across records".into());
            }
        }
    }
    tax_view(m)?;
    let mut employees = BTreeSet::new();
    for e in &m.employees {
        if !["", "Active", "Resigned", "Fired"].contains(&e.status.as_str()) || !["", "Employee", "Employer"].contains(&e.tax_payer.as_str()) { return Err("Invalid employee status or tax payer".into()); }
        if e.id.is_empty() || !employees.insert(&e.id) {
            return Err("Duplicate employee identifier".into());
        }
        let mut names = BTreeSet::new();
        for field in &e.identifiers { if field.name.trim().is_empty() || field.value.trim().is_empty() || !names.insert(field.name.trim().to_lowercase()) { return Err("Each employee identifier needs a unique name and a full value".into()); } }
        payroll(
            e,
            m.business
                .as_ref()
                .map(|b| b.period.as_str())
                .unwrap_or("2026-01"),
        )?;
    }
    Ok(())
}
fn valid_period(p: &str) -> Result<(), String> {
    NaiveDate::parse_from_str(&format!("{p}-01"), "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| "Period must be YYYY-MM".into())
}
pub fn merge(target: &Metadata, source: &Metadata, rollover: bool) -> Result<Metadata, String> {
    validate(target)?;
    validate(source)?;
    let a = target
        .business
        .as_ref()
        .ok_or("Destination needs business settings")?;
    let b = source
        .business
        .as_ref()
        .ok_or("Source needs business settings")?;
    if a.company_id != b.company_id
        || a.currency != b.currency
        || a.country != b.country
        || a.accounting != b.accounting
    {
        return Err("Company, currency, jurisdiction or accounting method mismatch".into());
    }
    if rollover && b.period >= a.period {
        return Err("Rollover source must precede destination period".into());
    }
    let mut next = target.clone();
    for incoming in &source.records {
        if let Some(existing) = next.records.iter_mut().find(|r| r.id == incoming.id) {
            if incoming.revision != existing.revision
                || incoming.amount != existing.amount
                || incoming.period != existing.period
                || incoming.category != existing.category
                || incoming.party != existing.party
                || incoming.description != existing.description
                || incoming.due != existing.due
                || incoming.calculation != existing.calculation
                || incoming.managed_key != existing.managed_key
                || incoming.disputed != existing.disputed
            {
                return Err(format!("Revision conflict for {}. Reconcile the source before import; no changes were applied.",incoming.id));
            }
            for p in &incoming.payments {
                if let Some(old) = existing.payments.iter().find(|v| v.id == p.id) {
                    if old != p {
                        return Err("Payment revision conflict".into());
                    }
                } else {
                    existing.payments.push(p.clone());
                }
            }
        } else if !rollover || money(&balance(incoming, "")?.outstanding)? > Decimal::ZERO {
            next.records.push(incoming.clone());
        }
    }
    next.lineage.insert(source.workbook_id.clone(), 1);
    next.audit
        .push(format!("Imported {} ({})", source.workbook_id, b.period));
    validate(&next)?;
    Ok(next)
}
pub fn summary(m: &Metadata, from: &str, to: &str) -> Result<serde_json::Value, String> {
    valid_period(from)?;
    valid_period(to)?;
    if from > to {
        return Err("Reporting dates are reversed".into());
    }
    let mut revenue = Decimal::ZERO;
    let mut expenses = Decimal::ZERO;
    let mut receipts = Decimal::ZERO;
    let mut paid = Decimal::ZERO;
    let mut receivable = Decimal::ZERO;
    let mut payable = Decimal::ZERO;
    for r in &m.records {
        if r.period.as_str() > to {
            continue;
        }
        let income = r.category == "Income";
        let amt = money(&r.amount)?;
        if r.period.as_str() >= from {
            if income {
                revenue += amt;
            } else if matches!(
                r.category.as_str(),
                "Expense" | "Fuel" | "Vehicle repairs" | "Payroll" | "Payroll withholding" | "Employee withholding" | "Employer contribution" | "Employer tax" | "Tax" | "Refund"
            ) {
                expenses += amt;
            }
        }
        let mut settled = Decimal::ZERO;
        for p in &r.payments {
            let month = p.date.get(..7).ok_or("Invalid payment date")?;
            if month <= to {
                settled += money(&p.amount)?;
            }
            if month >= from && month <= to {
                if income {
                    receipts += money(&p.amount)?;
                } else {
                    paid += money(&p.amount)?;
                }
            }
        }
        if income {
            receivable += amt - settled;
        } else {
            payable += amt - settled;
        }
    }
    Ok(
        serde_json::json!({"revenue":revenue.to_string(),"expenses":expenses.to_string(),"profit":(revenue-expenses).to_string(),"cash_received":receipts.to_string(),"cash_paid":paid.to_string(),"cash_movement":(receipts-paid).to_string(),"receivables":receivable.to_string(),"payables":payable.to_string()}),
    )
}
pub fn days_in_month(period: &str) -> Result<u32, String> {
    valid_period(period)?;
    let d = NaiveDate::parse_from_str(&format!("{period}-01"), "%Y-%m-%d").unwrap();
    let next = if d.month() == 12 {
        NaiveDate::from_ymd_opt(d.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(d.year(), d.month() + 1, 1)
    }
    .ok_or("Invalid month")?;
    Ok((next - d).num_days() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn meta(period: &str) -> Metadata {
        Metadata {
            business: Some(Business {
                company_id: "acme".into(),
                name: "Acme".into(),
                country: "ZA".into(),
                region: "".into(),
                currency: "ZAR".into(),
                kind: "General".into(),
                period: period.into(),
                year_start: 1,
                accounting: "Accrual".into(),
                registration: "".into(),
                tax_registration: "".into(),
            }),
            ..Default::default()
        }
    }
    fn record(period: &str, category: &str, amount: &str) -> Record {
        Record {
            id: Uuid::new_v4().to_string(),
            revision: 1,
            period: period.into(),
            category: category.into(),
            party: "Party".into(),
            description: "Test".into(),
            amount: amount.into(),
            due: "".into(),
            payments: vec![],
            disputed: false,
            source_workbook: "source".into(),
            ..Default::default()
        }
    }
    fn payment(amount: &str, date: &str) -> Payment {
        Payment {
            id: Uuid::new_v4().to_string(),
            amount: amount.into(),
            date: date.into(),
            method: "Bank".into(),
            reference: "r".into(),
            notes: "".into(),
            reversal_of: None,
        }
    }
    #[test]
    fn decimal_exact() {
        assert_eq!(
            money("0.10").unwrap() + money("0.20").unwrap(),
            money("0.30").unwrap()
        );
        assert!(money("1.001").is_err());
    }
    #[test]
    fn rollover_no_double_revenue() {
        let mut oct = meta("2026-10");
        let mut r = record("2026-10", "Income", "10000");
        r.payments.push(payment("4000", "2026-10-10"));
        oct.records.push(r);
        let mut nov = merge(&meta("2026-11"), &oct, true).unwrap();
        nov.records[0].payments.push(payment("2000", "2026-11-02"));
        let s = summary(&nov, "2026-11", "2026-11").unwrap();
        assert_eq!(s["revenue"], "0");
        assert_eq!(s["cash_received"], "2000");
        assert_eq!(s["receivables"], "4000");
    }
    #[test]
    fn twelve_months_out_of_order_idempotent() {
        let mut all = meta("2026-12");
        let mut files = vec![];
        for month in 1..=12 {
            let p = format!("2026-{month:02}");
            let mut m = meta(&p);
            for (cat, amount, paid) in [
                ("Income", "10000", "4000"),
                ("Expense", "2000", "2000"),
                ("Payroll", "3000", "1000"),
                ("Employee withholding", "400", "100"),
                ("Employer contribution", "100", "50"),
            ] {
                let mut r = record(&p, cat, amount);
                r.payments.push(payment(paid, &format!("{p}-20")));
                m.records.push(r);
            }
            files.push(m);
        }
        for m in files.iter().rev() {
            all = merge(&all, m, false).unwrap();
        }
        let before = summary(&all, "2026-01", "2026-12").unwrap();
        for m in &files {
            all = merge(&all, m, false).unwrap();
        }
        assert_eq!(all.records.len(), 60);
        assert_eq!(summary(&all, "2026-01", "2026-12").unwrap(), before);
        assert_eq!(before["profit"], "54000");
        assert_eq!(before["receivables"], "72000");
        assert_eq!(before["payables"], "28200");
        let next = merge(&meta("2027-01"), &all, true).unwrap();
        assert_eq!(
            summary(&next, "2027-01", "2027-01").unwrap()["revenue"],
            "0"
        );
    }
    #[test]
    fn reject_revision_without_mutation() {
        let mut m = meta("2026-01");
        m.records.push(record("2026-01", "Income", "10"));
        let mut changed = m.clone();
        changed.records[0].revision = 2;
        assert!(merge(&m, &changed, false).is_err());
        assert_eq!(m.records[0].revision, 1);
    }
    #[test]
    fn reversals_preserve_history() {
        let mut r = record("2026-01", "Payroll", "8000");
        let p = payment("5000", "2026-02-01");
        r.payments.push(p.clone());
        assert_eq!(balance(&r, "").unwrap().outstanding, "3000");
        let mut reverse = payment("-5000", "2026-02-02");
        reverse.reversal_of = Some(p.id);
        r.payments.push(reverse);
        assert_eq!(balance(&r, "").unwrap().outstanding, "8000");
    }
    #[test]
    fn leap_calendar() {
        assert_eq!(days_in_month("2024-02").unwrap(), 29);
        assert_eq!(days_in_month("2026-02").unwrap(), 28);
    }
    #[test]
    fn hourly_and_monthly() {
        let mut e = Employee {
            id: "1".into(),
            name: "A".into(),
            role: "".into(),
            structure: "Hourly".into(),
            rate: "100".into(),
            standard_hours: "8".into(),
            overtime_multiplier: "1.5".into(),
            days: vec![Day {
                date: "2026-01-01".into(),
                status: "Present".into(),
                hours: "4".into(),
                overtime: "2".into(),
                extra: "10".into(),
                deduction: "0".into(),
                note: "".into(),
            }],
            contributions: vec![],
            manual_tax: "0".into(),
            ..Default::default()
        };
        assert_eq!(
            money(&payroll(&e, "2026-01").unwrap().gross).unwrap(),
            money("710.00").unwrap()
        );
        e.structure = "Monthly".into();
        e.rate = "10000".into();
        e.days.clear();
        assert_eq!(
            money(&payroll(&e, "2026-01").unwrap().net).unwrap(),
            money("10000.00").unwrap()
        );
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Vehicle {
    pub start_odometer: String,
    pub end_odometer: String,
    pub units: String,
    pub registration: String,
    pub employee: String,
    pub price_per_litre: String,
    pub litres: String,
    pub part_name: String,
    pub part_number: String,
    pub part_amount: String,
    pub labour_amount: String,
    pub part: TransactionTax,
    pub labour: TransactionTax,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TaxBand {
    pub lower: String,
    pub upper: String,
    pub percent: String,
    // Empty fixed means carry forward the tax at the preceding band boundary.
    pub fixed: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ManualDeduction {
    pub id: String,
    pub name: String,
    pub percent: String,
    pub fixed: String,
    pub target: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EmployeeExtra {
    pub id: String,
    pub number: String,
    pub employee: String,
    pub name: String,
    pub kind: String,
    pub base: String,
    pub percent: String,
    pub fixed: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Pricing { pub enabled: bool, pub gross: String, pub discount_percent: String, pub discount_fixed: String }
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Identifier { pub name: String, pub value: String }
pub fn calculated_amount(r: &Record) -> Result<Decimal,String> {
    let base = if r.category == "Fuel" { nonnegative(&r.vehicle.price_per_litre)? * nonnegative(&r.vehicle.litres)? }
      else if r.category == "Vehicle repairs" { nonnegative(&r.vehicle.part_amount)? + nonnegative(&r.vehicle.labour_amount)? }
      else { nonnegative(&r.pricing.gross)? };
    let discount = if ["Income","Fuel"].contains(&r.category.as_str()) {percent_fixed(base,&r.pricing.discount_percent,&r.pricing.discount_fixed)?}else{Decimal::ZERO};
    if nonnegative(&r.pricing.discount_percent)? > Decimal::from(100) || discount > base { return Err("Discount cannot exceed the gross total".into()); }
    Ok(rounded(base-discount))
}
pub fn fuel_distance(r: &Record) -> Result<Option<(Decimal,Decimal)>,String> {
    let v=&r.vehicle;
    if v.start_odometer.is_empty() && v.end_odometer.is_empty() { return Ok(None); }
    if v.start_odometer.is_empty() || v.end_odometer.is_empty() { return Err("Enter both starting and ending odometer readings".into()); }
    let start=nonnegative(&v.start_odometer)?; let end=nonnegative(&v.end_odometer)?;
    if end<start { return Err("Ending odometer cannot be less than starting odometer".into()); }
    let quantity=nonnegative(&v.litres)?;
    if quantity<=Decimal::ZERO { return Err("Fuel quantity must be positive to calculate consumption".into()); }
    Ok(Some((end-start,(end-start)/quantity)))
}
