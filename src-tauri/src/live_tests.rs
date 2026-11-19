//! V2 regressions use synthetic records only; no real payroll or payment data.
use crate::{files, finance::*, live, templates};
use ironcalc::base::UserModel;
use rust_decimal::Decimal;
use serde_json::json;

fn business(period: &str) -> Business {
    Business {
        company_id: "v2-regression-company".into(),
        name: "V2 Synthetic Company".into(),
        country: "South Africa".into(),
        currency: "ZAR".into(),
        kind: "General".into(),
        period: period.into(),
        year_start: 1,
        accounting: "Accrual".into(),
        ..Default::default()
    }
}
fn metadata() -> Metadata {
    Metadata {
        business: Some(business("2026-11")),
        template: "General Business".into(),
        ..Default::default()
    }
}
fn employee(id: &str) -> Employee {
    Employee {
        id: id.into(),
        name: format!("Employee {id}"),
        structure: "Monthly".into(),
        rate: "10000".into(),
        standard_hours: "8".into(),
        overtime_multiplier: "1.5".into(),
        manual_tax: "0".into(),
        tax_percent: "0".into(),
        tax_payer: "Employee".into(),
        status: "Active".into(),
        ..Default::default()
    }
}
fn contribution_pool() -> Contribution {
    Contribution {
        name: "Retirement fund".into(),
        total_percent: "10".into(),
        total_fixed: "100".into(),
        employee_percent: "40".into(),
        employer_percent: "60".into(),
        employee_fixed: "5".into(),
        employer_fixed: "7".into(),
        base: "0".into(),
        split_pool: true,
        ..Default::default()
    }
}
fn day(date: &str, hours: &str) -> Day {
    Day {
        date: date.into(),
        status: "Present".into(),
        hours: hours.into(),
        overtime: "0".into(),
        extra: "0".into(),
        deduction: "0".into(),
        note: "Synthetic attendance".into(),
    }
}
fn record(category: &str, amount: &str) -> Record {
    Record {
        id: uuid::Uuid::new_v4().to_string(),
        revision: 1,
        period: "2026-11".into(),
        category: category.into(),
        party: "Synthetic party".into(),
        description: "Synthetic record".into(),
        amount: amount.into(),
        ..Default::default()
    }
}
fn payment(amount: &str, date: &str) -> Payment {
    Payment {
        id: uuid::Uuid::new_v4().to_string(),
        amount: amount.into(),
        date: date.into(),
        method: "Test only".into(),
        reference: "TEST".into(),
        reversal_of: None,
        notes: String::new(),
    }
}
fn eq_amount(actual: &str, expected: &str) {
    assert_eq!(money(actual).unwrap(), money(expected).unwrap());
}
fn obligation<'a>(m: &'a Metadata, suffix: &str) -> &'a Record {
    m.records
        .iter()
        .find(|r| r.managed_key == format!("2026-11:{suffix}"))
        .expect("generated obligation")
}
fn employer_tax_obligations(m: &Metadata) -> Decimal {
    m.records
        .iter()
        .filter(|r| r.period == "2026-11" && (r.category == "Employer tax" || r.category == "Tax"))
        .map(|r| money(&r.amount).unwrap())
        .sum()
}
fn sheet_contains(m: &UserModel, sheet: u32, text: &str) -> bool {
    (1..=50).any(|r| {
        (1..=16).any(|c| m.get_formatted_cell_value(sheet, r, c).unwrap_or_default() == text)
    })
}

#[test]
fn pooled_contributions_split_total_and_add_each_fixed_amount() {
    let c = contribution_pool();
    let result = contribution(&c, money("10000").unwrap()).unwrap();
    eq_amount(&result.employee, "445");
    eq_amount(&result.employer, "667");
    eq_amount(&result.total, "1112");
    assert_eq!(result.name, "Retirement fund");
    let mut fixed = c.clone();
    fixed.total_percent = "0".into();
    fixed.total_fixed = "200".into();
    eq_amount(
        &contribution(&fixed, money("10000").unwrap()).unwrap().total,
        "212",
    );
    fixed.total_fixed = "0".into();
    fixed.employee_percent = "0".into();
    fixed.employer_percent = "0".into();
    eq_amount(&contribution(&fixed, Decimal::ZERO).unwrap().total, "12");
}

#[test]
fn pooled_contribution_rounding_preserves_the_total_pool() {
    let mut c = contribution_pool();
    c.total_percent = "0".into();
    c.total_fixed = "0.01".into();
    c.employee_percent = "50".into();
    c.employer_percent = "50".into();
    c.employee_fixed = "0".into();
    c.employer_fixed = "0".into();
    let result = contribution(&c, Decimal::ZERO).unwrap();
    eq_amount(&result.total, "0.01");
    assert_eq!(
        money(&result.employee).unwrap() + money(&result.employer).unwrap(),
        money("0.01").unwrap()
    );
}

#[test]
fn contribution_validation_rejects_unallocated_pool_and_negative_values() {
    let mut c = contribution_pool();
    c.employer_percent = "50".into();
    assert!(contribution(&c, money("10000").unwrap()).is_err());
    c.employer_percent = "60".into();
    c.total_fixed = "-1".into();
    assert!(contribution(&c, money("10000").unwrap()).is_err());
    c.total_fixed = "100".into();
    c.employee_fixed = "0.001".into();
    assert!(contribution(&c, money("10000").unwrap()).is_err());
    c.employee_fixed = "0".into();
    c.name.clear();
    assert!(contribution(&c, money("10000").unwrap()).is_err());
}

#[test]
fn legacy_employee_json_retains_direct_percentages_and_fixed_tax() {
    let e: Employee = serde_json::from_value(json!({
        "id":"old", "name":"Legacy Employee", "role":"Test", "structure":"Monthly", "rate":"10000",
        "standard_hours":"8", "overtime_multiplier":"1.5", "manual_tax":"300",
        "contributions":[{"name":"Legacy fund", "employee_percent":"1", "employer_percent":"2", "employee_fixed":"5", "employer_fixed":"7", "cap":null}]
    })).unwrap();
    assert_eq!(e.status, "Active");
    assert_eq!(e.tax_payer, "Employee");
    assert!(!e.contributions[0].split_pool);
    let p = payroll(&e, "2026-11").unwrap();
    eq_amount(&p.tax, "300");
    eq_amount(&p.employee_contributions, "105");
    eq_amount(&p.employer_contributions, "207");
    eq_amount(&p.net, "9595");
}

#[test]
fn employer_borne_tax_increases_cost_without_deducting_net_salary() {
    let mut e = employee("one");
    e.tax_percent = "10".into();
    e.manual_tax = "50".into();
    e.contributions.push(contribution_pool());
    let own = payroll(&e, "2026-11").unwrap();
    eq_amount(&own.tax, "1050");
    eq_amount(&own.net, "8505");
    eq_amount(&own.employer_cost, "10667");
    e.tax_payer = "Employer".into();
    let employer = payroll(&e, "2026-11").unwrap();
    eq_amount(&employer.tax, "0");
    eq_amount(&employer.employer_tax, "1050");
    eq_amount(&employer.net, "9555");
    eq_amount(&employer.employer_cost, "11717");
}

#[test]
fn attendance_recalculates_live_obligations_without_duplicate_posting() {
    let mut m = metadata();
    let mut e = employee("hourly");
    e.structure = "Hourly".into();
    e.rate = "20".into();
    e.tax_percent = "10".into();
    e.days.push(day("2026-11-01", "8"));
    m.employees.push(e);
    live::sync_records(&mut m).unwrap();
    eq_amount(&obligation(&m, "payroll:hourly").amount, "144");
    eq_amount(&obligation(&m, "employee-tax:hourly").amount, "16");
    let ids: Vec<_> = m.records.iter().map(|r| r.id.clone()).collect();
    live::sync_records(&mut m).unwrap();
    assert_eq!(
        ids,
        m.records.iter().map(|r| r.id.clone()).collect::<Vec<_>>()
    );
    m.employees[0].days.push(day("2026-11-02", "8"));
    live::sync_records(&mut m).unwrap();
    eq_amount(&obligation(&m, "payroll:hourly").amount, "288");
    eq_amount(&obligation(&m, "employee-tax:hourly").amount, "32");
    assert_eq!(
        ids,
        m.records.iter().map(|r| r.id.clone()).collect::<Vec<_>>()
    );
    eq_amount(
        summary(&m, "2026-11", "2026-11").unwrap()["expenses"]
            .as_str()
            .unwrap(),
        "320",
    );
}

#[test]
fn live_payroll_preserves_prior_payment_when_attendance_increases() {
    let mut m = metadata();
    let mut e = employee("one");
    e.structure = "Hourly".into();
    e.rate = "10".into();
    e.days.push(day("2026-11-01", "8"));
    m.employees.push(e);
    live::sync_records(&mut m).unwrap();
    let r = m
        .records
        .iter_mut()
        .find(|r| r.managed_key.ends_with("payroll:one"))
        .unwrap();
    let p = payment("80", "2026-11-02");
    r.payments.push(p.clone());
    m.employees[0].days.push(day("2026-11-02", "8"));
    live::sync_records(&mut m).unwrap();
    let r = obligation(&m, "payroll:one");
    assert_eq!(r.payments, vec![p]);
    let b = balance(r, "2026-11-03").unwrap();
    eq_amount(&b.paid, "80");
    eq_amount(&b.outstanding, "80");
    assert_eq!(b.status, "Partially settled");
}

#[test]
fn reducing_paid_payroll_is_rejected_until_payment_is_reversed() {
    let mut m = metadata();
    m.employees.push(employee("one"));
    live::sync_records(&mut m).unwrap();
    let r = m
        .records
        .iter_mut()
        .find(|r| r.managed_key.ends_with("payroll:one"))
        .unwrap();
    let p = payment("10000", "2026-11-13");
    r.payments.push(p.clone());
    let mut draft = m.clone();
    draft.employees[0].rate = "9000".into();
    assert!(live::sync_records(&mut draft).is_err());
    // Failed edit is applied to a draft, never to the accepted document.
    eq_amount(&obligation(&m, "payroll:one").amount, "10000");
    let r = m
        .records
        .iter_mut()
        .find(|r| r.managed_key.ends_with("payroll:one"))
        .unwrap();
    let mut reversal = payment("-10000", "2026-11-14");
    reversal.reversal_of = Some(p.id);
    r.payments.push(reversal);
    m.employees[0].rate = "9000".into();
    live::sync_records(&mut m).unwrap();
    eq_amount(
        &balance(obligation(&m, "payroll:one"), "2026-11-14")
            .unwrap()
            .outstanding,
        "9000",
    );
}

#[test]
fn settled_date_follows_active_payments_and_clears_when_marked_unpaid() {
    let mut r = record("Expense", "100");
    r.due = "2026-11-10".into();
    assert_eq!(balance(&r, "2026-11-13").unwrap().status, "Overdue");
    let p1 = payment("30", "2026-11-11");
    let p2 = payment("70", "2026-11-13");
    r.payments.extend([p1.clone(), p2.clone()]);
    let paid = serde_json::to_value(balance(&r, "2026-11-13").unwrap()).unwrap();
    assert_eq!(paid["status"], "Settled");
    assert_eq!(paid["paid_date"], "2026-11-13");
    for p in [p1, p2] {
        let mut reversal = payment(&format!("-{}", p.amount), "2026-11-14");
        reversal.reversal_of = Some(p.id);
        r.payments.push(reversal);
    }
    let unpaid = serde_json::to_value(balance(&r, "2026-11-14").unwrap()).unwrap();
    assert_eq!(unpaid["status"], "Overdue");
    assert_eq!(unpaid["paid_date"], "");
    assert_eq!(r.payments.len(), 4);
    r.payments.push(payment("100", "2026-11-15"));
    assert_eq!(
        serde_json::to_value(balance(&r, "2026-11-15").unwrap()).unwrap()["paid_date"],
        "2026-11-15"
    );
}

#[test]
fn manual_transaction_tax_and_deductions_feed_employer_liability() {
    let mut m = metadata();
    m.business_taxes.push(TaxRule {
        id: "business-tax".into(),
        name: "Company tax".into(),
        base: "2000".into(),
        percent: "10".into(),
        fixed: "50".into(),
    ..Default::default()
});
    let mut r = record("Income", "1000");
    r.calculation = TransactionTax {
        tax_percent: "15".into(),
        tax_fixed: "10".into(),
        deductible_percent: "2".into(),
        deductible_fixed: "5".into(),
    ..Default::default()
};
    m.records.push(r);
    let view = live::tax_view(&m).unwrap();
    eq_amount(&view.employer_tax, "410");
    eq_amount(&view.deductions, "25");
    eq_amount(&view.net_employer_tax, "385");
    live::sync_records(&mut m).unwrap();
    assert_eq!(employer_tax_obligations(&m), money("385").unwrap());
    eq_amount(
        summary(&m, "2026-11", "2026-11").unwrap()["payables"]
            .as_str()
            .unwrap(),
        "385",
    );
    let before = serde_json::to_value(live::tax_view(&m).unwrap()).unwrap();
    live::sync_records(&mut m).unwrap();
    assert_eq!(
        before,
        serde_json::to_value(live::tax_view(&m).unwrap()).unwrap()
    );
}

#[test]
fn excess_transaction_deductions_do_not_make_negative_tax_obligations() {
    let mut m = metadata();
    let mut r = record("Expense", "100");
    r.calculation.tax_fixed = "10".into();
    r.calculation.deductible_fixed = "30".into();
    m.records.push(r);
    let view = live::tax_view(&m).unwrap();
    eq_amount(&view.net_employer_tax, "0");
    eq_amount(&view.unused_deductions, "20");
    live::sync_records(&mut m).unwrap();
    assert_eq!(employer_tax_obligations(&m), Decimal::ZERO);
    validate(&m).unwrap();
}

#[test]
fn legacy_tax_and_deductible_have_consistent_report_and_payable_totals() {
    let mut m = metadata();
    m.records.push(record("Tax", "100"));
    let mut claim = record("Income", "200");
    claim.calculation.deductible_fixed = "20".into();
    m.records.push(claim);
    live::sync_records(&mut m).unwrap();
    let view = live::tax_view(&m).unwrap();
    eq_amount(&view.net_employer_tax, "80");
    eq_amount(
        summary(&m, "2026-11", "2026-11").unwrap()["payables"]
            .as_str()
            .unwrap(),
        "80",
    );
}

#[test]
fn employee_and_employer_contribution_totals_remain_separate_and_named() {
    let mut m = metadata();
    let mut e = employee("one");
    e.contributions.push(contribution_pool());
    e.tax_percent = "10".into();
    m.employees.push(e);
    let mut c = contribution_pool();
    c.name = "Business insurance".into();
    c.base = "1000".into();
    c.employee_percent = "0".into();
    c.employer_percent = "100".into();
    c.employee_fixed = "0".into();
    c.employer_fixed = "0".into();
    m.business_contributions.push(c);
    let view = live::tax_view(&m).unwrap();
    eq_amount(&view.employee_tax, "1000");
    eq_amount(&view.employee_contributions, "445");
    eq_amount(&view.employer_contributions, "867");
    assert!(view
        .rows
        .iter()
        .any(|r| r.name == "Retirement fund" && r.source == "Employee one"));
    assert!(view.rows.iter().any(|r| r.name == "Business insurance"));
    live::sync_records(&mut m).unwrap();
    eq_amount(&obligation(&m, "employee-contributions:one").amount, "445");
    eq_amount(&obligation(&m, "employer-contributions:one").amount, "667");
    eq_amount(&obligation(&m, "business-contributions").amount, "200");
}

#[test]
fn legacy_posted_payroll_with_duplicate_employee_names_is_adopted_once() {
    let mut m = metadata();
    for id in ["one", "two"] {
        let mut e = employee(id);
        e.name = "Same name".into();
        m.employees.push(e);
        let mut r = record("Payroll", "10000");
        r.party = "Same name".into();
        r.description = format!("Payroll {id}");
        r.payments.push(payment("100", "2026-11-13"));
        m.records.push(r);
    }
    let originals: Vec<_> = m.records.iter().map(|r| r.id.clone()).collect();
    live::sync_records(&mut m).unwrap();
    assert_eq!(
        m.records.iter().filter(|r| r.category == "Payroll").count(),
        2
    );
    for id in originals {
        assert!(m
            .records
            .iter()
            .any(|r| r.id == id && r.payments.len() == 1 && !r.managed_key.is_empty()));
    }
}

#[test]
fn rollover_carries_active_employees_settings_and_old_outstanding_only() {
    let mut m = metadata();
    for (id, status) in [
        ("active", "Active"),
        ("resigned", "Resigned"),
        ("fired", "Fired"),
    ] {
        let mut e = employee(id);
        e.status = status.into();
        e.days.push(day("2026-11-03", "8"));
        e.tax_percent = "1".into();
        e.contributions.push(contribution_pool());
        m.employees.push(e);
    }
    m.business_taxes.push(TaxRule {
        id: "tax".into(),
        name: "Recurring tax".into(),
        base: "0".into(),
        percent: "0".into(),
        fixed: "20".into(),
    ..Default::default()
});
    live::sync_records(&mut m).unwrap();
    let settled_id = obligation(&m, "payroll:active").id.clone();
    let r = m.records.iter_mut().find(|r| r.id == settled_id).unwrap();
    r.payments.push(payment(&r.amount, "2026-11-30"));
    let source = serde_json::to_value(&m).unwrap();
    let next = live::rollover(&m, business("2026-12")).unwrap();
    assert_eq!(next.employees.len(), 1);
    assert_eq!(next.employees[0].id, "active");
    assert!(next.employees[0].days.is_empty());
    assert_eq!(next.employees[0].contributions[0].name, "Retirement fund");
    assert_eq!(next.employees[0].tax_percent, "1");
    assert_eq!(next.business_taxes.len(), 1);
    assert!(!next.records.iter().any(|r| r.id == settled_id));
    assert!(next
        .records
        .iter()
        .any(|r| r.period == "2026-11" && r.description == "Payroll resigned"));
    assert!(next
        .records
        .iter()
        .any(|r| r.managed_key == "2026-12:payroll:active"));
    assert_eq!(source, serde_json::to_value(&m).unwrap());
}

#[test]
fn hourly_rollover_retains_fixed_deductions_and_reports_shortfall_until_earned() {
    let mut m = metadata();
    let mut e = employee("hourly");
    e.structure = "Hourly".into();
    e.rate = "20".into();
    e.manual_tax = "50".into();
    e.days.push(day("2026-11-02", "8"));
    let mut c = contribution_pool();
    c.total_fixed = "0".into();
    c.total_percent = "0".into();
    c.employee_fixed = "10".into();
    c.employer_fixed = "5".into();
    e.contributions.push(c);
    m.employees.push(e);
    live::sync_records(&mut m).unwrap();
    let mut next = live::rollover(&m, business("2026-12")).unwrap();
    let p = payroll(&next.employees[0], "2026-12").unwrap();
    eq_amount(&p.gross, "0");
    eq_amount(&p.net, "0");
    eq_amount(&p.deduction_shortfall, "60");
    eq_amount(&p.tax, "50");
    eq_amount(&p.employee_contributions, "10");
    assert!(p.explanation.iter().any(|line| line.contains("Deduction shortfall")));
    next.employees[0].days.push(day("2026-12-01", "8"));
    live::sync_records(&mut next).unwrap();
    let p = payroll(&next.employees[0], "2026-12").unwrap();
    eq_amount(&p.net, "100");
    eq_amount(&p.deduction_shortfall, "0");
    validate(&next).unwrap();
}

#[test]
fn deleting_contribution_rebuilds_sheets_without_stale_named_rows() {
    let (mut model, mut m) =
        templates::create("General Business", Some(business("2026-11"))).unwrap();
    let mut e = employee("one");
    let mut c = contribution_pool();
    c.name = "UniqueDeletedContribution".into();
    e.contributions.push(c);
    m.employees.push(e);
    live::sync_records(&mut m).unwrap();
    templates::sync(&mut model, &m).unwrap();
    let sheets = model.get_model().workbook.worksheets.len() as u32;
    assert!(
        (0..sheets).any(|s| sheet_contains(&model, s, "UniqueDeletedContribution")),
        "Named contribution must be visible in a worksheet"
    );
    m.employees[0].contributions.clear();
    live::sync_records(&mut m).unwrap();
    templates::sync(&mut model, &m).unwrap();
    assert!(
        !(0..sheets).any(|s| sheet_contains(&model, s, "UniqueDeletedContribution")),
        "Removed contribution must not leave stale sheet data"
    );
    eq_amount(&obligation(&m, "employee-contributions:one").amount, "0");
    eq_amount(&obligation(&m, "employer-contributions:one").amount, "0");
}

#[test]
fn projection_totals_are_visible_after_data_and_recalculate_on_removal() {
    let (mut model, mut m) =
        templates::create("General Business", Some(business("2026-11"))).unwrap();
    m.records.push(record("Income", "100.10"));
    m.records.push(record("Income", "200.20"));
    m.employees.push(employee("one"));
    live::sync_records(&mut m).unwrap();
    templates::sync(&mut model, &m).unwrap();
    assert!(model
        .get_formatted_cell_value(1, 6, 1)
        .unwrap()
        .to_lowercase()
        .contains("total"));
    eq_amount(
        &model
            .get_formatted_cell_value(1, 6, 4)
            .unwrap()
            .replace(',', ""),
        "300.30",
    );
    assert!(model
        .get_formatted_cell_value(3, 5, 1)
        .unwrap()
        .to_lowercase()
        .contains("total"));
    eq_amount(
        &model
            .get_formatted_cell_value(3, 5, 3)
            .unwrap()
            .replace(',', ""),
        "10000",
    );
    m.records.remove(1);
    live::sync_records(&mut m).unwrap();
    templates::sync(&mut model, &m).unwrap();
    assert!(model
        .get_formatted_cell_value(1, 5, 1)
        .unwrap()
        .to_lowercase()
        .contains("total"));
    eq_amount(
        &model
            .get_formatted_cell_value(1, 5, 4)
            .unwrap()
            .replace(',', ""),
        "100.10",
    );
    assert_eq!(model.get_formatted_cell_value(1, 6, 4).unwrap(), "");
}

#[test]
fn deleting_unpaid_employee_removes_only_their_live_obligations() {
    let mut m = metadata();
    m.employees.extend([employee("one"), employee("two")]);
    live::sync_records(&mut m).unwrap();
    m.employees.retain(|e| e.id != "one");
    live::sync_records(&mut m).unwrap();
    assert!(!m.records.iter().any(|r| r.managed_key.ends_with(":one")));
    eq_amount(&obligation(&m, "payroll:two").amount, "10000");
}

#[test]
fn updated_tax_inputs_contributions_and_payments_survive_xlsx_and_ods() {
    let (mut model, mut m) =
        templates::create("General Business", Some(business("2026-11"))).unwrap();
    let mut e = employee("one");
    e.tax_percent = "7.5".into();
    e.manual_tax = "25".into();
    e.tax_payer = "Employer".into();
    e.status = "Resigned".into();
    e.contributions.push(contribution_pool());
    m.employees.push(e);
    let mut r = record("Income", "1000");
    r.calculation = TransactionTax {
        tax_percent: "15".into(),
        tax_fixed: "10".into(),
        deductible_percent: "1".into(),
        deductible_fixed: "3".into(),
    ..Default::default()
};
    r.payments.push(payment("250", "2026-11-13"));
    m.records.push(r);
    live::sync_records(&mut m).unwrap();
    templates::sync(&mut model, &m).unwrap();
    let expected = serde_json::to_value(&m).unwrap();
    let tmp = tempfile::tempdir().unwrap();
    for ext in ["xlsx", "ods"] {
        let path = tmp.path().join(format!("v2-business.{ext}"));
        files::save(path.to_str().unwrap(), &model, &m).unwrap();
        let (loaded, meta) = files::load(path.to_str().unwrap()).unwrap();
        assert_eq!(
            serde_json::to_value(meta).unwrap(),
            expected,
            "{ext} metadata must retain every input and payment"
        );
        assert!(
            loaded.get_model().workbook.worksheets.len() >= 13,
            "Separate employer and employee tax sheets"
        );
        assert!(
            (0..loaded.get_model().workbook.worksheets.len() as u32).any(|s| sheet_contains(
                &loaded,
                s,
                "Retirement fund"
            ))
        );
    }
}

#[test]
fn imported_current_period_contribution_snapshot_is_not_recalculated_from_empty_settings() {
    let mut source = metadata();
    let mut c = contribution_pool();
    c.total_percent = "0".into();
    c.total_fixed = "200".into();
    c.employee_percent = "0".into();
    c.employer_percent = "100".into();
    c.employee_fixed = "0".into();
    c.employer_fixed = "0".into();
    source.business_contributions.push(c);
    live::sync_records(&mut source).unwrap();
    let source_record = obligation(&source, "business-contributions").clone();
    let target = metadata();
    let mut consolidated = merge(&target, &source, false).unwrap();
    live::sync_records(&mut consolidated).unwrap();
    assert_eq!(consolidated.records.iter().find(|r|r.id==source_record.id).unwrap(), &source_record);
    eq_amount(summary(&consolidated,"2026-11","2026-11").unwrap()["payables"].as_str().unwrap(),"200");
}

#[test]
fn local_deductions_do_not_reduce_imported_tax_snapshots_in_the_report() {
    let mut source = metadata();
    source.business_taxes.push(TaxRule {
        id: "source-tax".into(),
        name: "Source assessed tax".into(),
        base: "0".into(),
        percent: "0".into(),
        fixed: "100".into(),
    ..Default::default()
});
    live::sync_records(&mut source).unwrap();
    let source_tax = obligation(&source, "employer-rule:source-tax").clone();
    let mut target = metadata();
    target.business_taxes.push(TaxRule {
        id: "local-tax".into(),
        name: "Local assessed tax".into(),
        base: "0".into(),
        percent: "0".into(),
        fixed: "50".into(),
    ..Default::default()
});
    let mut claim = record("Income", "1000");
    claim.source_workbook = target.workbook_id.clone();
    claim.calculation.deductible_fixed = "200".into();
    target.records.push(claim);
    let mut combined = merge(&target, &source, false).unwrap();
    live::sync_records(&mut combined).unwrap();
    let view = live::tax_view(&combined).unwrap();
    eq_amount(&view.employer_tax, "150");
    eq_amount(&view.deductions, "200");
    eq_amount(&view.net_employer_tax, "100");
    eq_amount(&view.unused_deductions, "150");
    eq_amount(&view.employer_due, "100");
    assert_eq!(combined.records.iter().find(|r| r.id == source_tax.id).unwrap(), &source_tax);
    eq_amount(summary(&combined, "2026-11", "2026-11").unwrap()["payables"].as_str().unwrap(), "100");
    live::sync_records(&mut combined).unwrap();
    eq_amount(&live::tax_view(&combined).unwrap().net_employer_tax, "100");
    validate(&combined).unwrap();
}

