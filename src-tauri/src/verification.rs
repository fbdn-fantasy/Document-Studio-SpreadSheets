use crate::{files, finance::*, templates};
use ironcalc::base::{expressions::types::Area, UserModel};
fn model() -> UserModel<'static> {
    UserModel::new_empty("Verification", "en", "UTC", "en").unwrap()
}
#[test]
fn dependencies_cross_sheet_nonadjacent_and_circular() {
    let mut m = model();
    m.set_user_input(0, 1, 1, "10").unwrap();
    m.set_user_input(0, 8, 3, "20").unwrap();
    m.new_sheet().unwrap();
    m.rename_sheet(1, "Other values").unwrap();
    m.set_user_input(1, 2, 1, "30").unwrap();
    m.set_user_input(0, 30, 2, "=SUM(A1,C8,'Other values'!A2)")
        .unwrap();
    assert_eq!(m.get_formatted_cell_value(0, 30, 2).unwrap(), "60");
    m.set_user_input(1, 2, 1, "40").unwrap();
    assert_eq!(m.get_formatted_cell_value(0, 30, 2).unwrap(), "70");
    m.set_user_input(0, 1, 4, "=E1").unwrap();
    m.set_user_input(0, 1, 5, "=D1").unwrap();
    assert!(m
        .get_formatted_cell_value(0, 1, 4)
        .unwrap()
        .starts_with('#'));
}
#[test]
fn clipboard_adjusts_relative_references() {
    let mut m = model();
    m.set_user_input(0, 1, 1, "5").unwrap();
    m.set_user_input(0, 2, 1, "7").unwrap();
    m.set_user_input(0, 1, 2, "=A1+$A$1").unwrap();
    m.set_selected_sheet(0).unwrap();
    m.set_selected_cell(1, 2).unwrap();
    let clip = serde_json::to_value(m.copy_to_clipboard().unwrap()).unwrap();
    m.set_selected_cell(2, 2).unwrap();
    m.paste_from_clipboard(
        0,
        (1, 2, 1, 2),
        &serde_json::from_value(clip["data"].clone()).unwrap(),
        false,
    )
    .unwrap();
    assert_eq!(m.get_formatted_cell_value(0, 2, 2).unwrap(), "12");
    m.undo().unwrap();
    assert_eq!(m.get_cell_content(0, 2, 2).unwrap(), "");
    m.redo().unwrap();
    assert_eq!(m.get_formatted_cell_value(0, 2, 2).unwrap(), "12");
}
#[test]
fn sparse_limit_and_styles_roundtrip() {
    let mut m = model();
    m.set_user_input(0, 1048576, 16384, "42").unwrap();
    m.update_range_style(
        &Area {
            sheet: 0,
            row: 1048576,
            column: 16384,
            height: 1,
            width: 1,
        },
        "font.b",
        "true",
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sparse.xlsx");
    files::save(path.to_str().unwrap(), &m, &Metadata::default()).unwrap();
    assert!(std::fs::metadata(&path).unwrap().len() < 100000);
    let (loaded, _) = files::load(path.to_str().unwrap()).unwrap();
    assert_eq!(
        loaded.get_formatted_cell_value(0, 1048576, 16384).unwrap(),
        "42"
    );
    assert!(loaded.get_cell_style(0, 1048576, 16384).unwrap().font.b);
}
#[test]
fn every_template_creates_and_calculates() {
    for name in templates::NAMES {
        if name.contains("Company") || *name == "General Business" {
            continue;
        }
        let (mut m, _) = templates::create(name, None).unwrap();
        if *name == "Blank workbook" {
            continue;
        }
        m.set_user_input(0, 4, 1, "Item").unwrap();
        m.set_user_input(0, 4, 2, "10").unwrap();
        m.set_user_input(0, 4, 3, "20").unwrap();
        m.set_user_input(0, 4, 4, "5").unwrap();
        for c in 1..=7 {
            let value = m.get_formatted_cell_value(0, 4, c).unwrap();
            assert!(!value.starts_with('#'), "{name} col{c}: {value}");
        }
    }
}
fn business(period: &str) -> Business {
    Business {
        company_id: "verification-company".into(),
        name: "Verification Ltd".into(),
        country: "South Africa".into(),
        region: "Gauteng".into(),
        currency: "ZAR".into(),
        kind: "General".into(),
        period: period.into(),
        year_start: 1,
        accounting: "Accrual".into(),
        registration: "".into(),
        tax_registration: "".into(),
    }
}
fn rec(period: &str, cat: &str, amount: &str) -> Record {
    Record {
        id: uuid::Uuid::new_v4().to_string(),
        revision: 1,
        period: period.into(),
        category: cat.into(),
        party: "Verification party".into(),
        description: "Verification record".into(),
        amount: amount.into(),
        due: format!("{period}-28"),
        payments: vec![],
        disputed: false,
        source_workbook: "verification".into(),
        ..Default::default()
    }
}
fn pay(amount: &str, date: &str) -> Payment {
    Payment {
        id: uuid::Uuid::new_v4().to_string(),
        date: date.into(),
        amount: amount.into(),
        reference: "TEST-ONLY".into(),
        method: "Test".into(),
        reversal_of: None,
        notes: "Synthetic test payment; no real transaction".into(),
    }
}
#[test]
fn twelve_real_workbooks_roundtrip_consolidation() {
    let tmp = tempfile::tempdir().unwrap();
    let root = std::env::var("DSS_TEST_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or(tmp.path().to_path_buf());
    std::fs::create_dir_all(&root).unwrap();
    let mut paths = vec![];
    for month in 1..=12 {
        let period = format!("2026-{month:02}");
        let (mut m, mut meta) =
            templates::create("General Business", Some(business(&period))).unwrap();
        for (cat, amount, settled) in [
            ("Income", "10000", "4000"),
            ("Expense", "2000", "2000"),
            ("Payroll", "3000", "1000"),
            ("Tax", "400", "100"),
            ("Employer contribution", "100", "50"),
        ] {
            let mut r = rec(&period, cat, amount);
            // These records are authored in this workbook (matching the UI),
            // not imported snapshots from the helper's synthetic source ID.
            r.source_workbook = meta.workbook_id.clone();
            if cat == "Payroll" {
                r.description = format!("Payroll employee-{month}");
            }
            r.payments.push(pay(settled, &format!("{period}-20")));
            meta.records.push(r);
        }
        meta.employees.push(Employee {
            id: format!("employee-{month}"),
            name: "Synthetic employee".into(),
            role: "Test".into(),
            structure: "Monthly".into(),
            rate: "3000".into(),
            standard_hours: "8".into(),
            overtime_multiplier: "1.5".into(),
            days: vec![],
            contributions: vec![],
            manual_tax: "0".into(),
            ..Default::default()
        });
        crate::live::sync_records(&mut meta).unwrap();
        let original_record_ids: Vec<_> = meta.records.iter().map(|r| r.id.clone()).collect();
        crate::live::sync_records(&mut meta).unwrap();
        assert_eq!(original_record_ids, meta.records.iter().map(|r| r.id.clone()).collect::<Vec<_>>());
        templates::sync(&mut m, &meta).unwrap();
        let path = root.join(format!("2026-{month:02}.xlsx"));
        files::save(path.to_str().unwrap(), &m, &meta).unwrap();
        paths.push(path);
    }
    let (mut model, mut annual) =
        templates::create("General Business", Some(business("2026-12"))).unwrap();
    for path in paths.iter().rev() {
        let (_, source) = files::load(path.to_str().unwrap()).unwrap();
        annual = merge(&annual, &source, false).unwrap();
    }
    let before = summary(&annual, "2026-01", "2026-12").unwrap();
    assert_eq!(before["revenue"], "120000");
    assert_eq!(before["profit"], "54000");
    assert_eq!(before["receivables"], "72000");
    assert_eq!(before["payables"], "28200");
    for path in &paths {
        annual = merge(
            &annual,
            &files::load(path.to_str().unwrap()).unwrap().1,
            false,
        )
        .unwrap();
    }
    assert_eq!(before, summary(&annual, "2026-01", "2026-12").unwrap());
    assert_eq!(
        annual
            .records
            .iter()
            .filter(|r| money(&r.amount).unwrap() > rust_decimal::Decimal::ZERO)
            .count(),
        60
    );
    templates::sync(&mut model, &annual).unwrap();
    for ext in ["xlsx", "ods"] {
        let consolidated_path = root.join(format!("consolidated-2026.{ext}"));
        files::save(consolidated_path.to_str().unwrap(), &model, &annual).unwrap();
        let (_, reopened) = files::load(consolidated_path.to_str().unwrap()).unwrap();
        assert_eq!(summary(&reopened, "2026-01", "2026-12").unwrap(), before);
    }
    let next = merge(
        &Metadata {
            business: Some(business("2027-01")),
            ..Default::default()
        },
        &annual,
        true,
    )
    .unwrap();
    assert_eq!(
        summary(&next, "2027-01", "2027-01").unwrap()["revenue"],
        "0"
    );
}
#[test]
fn payroll_withholding_is_expensed_once() {
    let mut m = Metadata {
        business: Some(business("2026-11")),
        ..Default::default()
    };
    for (cat, amount) in [
        ("Payroll", "8500"),
        ("Payroll withholding", "1000"),
        ("Payroll withholding", "500"),
        ("Employer contribution", "100"),
    ] {
        let mut r = rec("2026-11", cat, amount);
        r.payments.push(pay(amount, "2026-11-30"));
        m.records.push(r);
    }
    let totals = summary(&m, "2026-11", "2026-11").unwrap();
    assert_eq!(totals["expenses"], "10100");
    assert_eq!(totals["cash_paid"], "10100");
    assert_eq!(totals["payables"], "0");
}
#[test]
fn negative_contributions_rejected() {
    let e = Employee {
        id: "test".into(),
        name: "Test".into(),
        role: "".into(),
        structure: "Monthly".into(),
        rate: "100".into(),
        standard_hours: "8".into(),
        overtime_multiplier: "1".into(),
        days: vec![],
        contributions: vec![Contribution {
            name: "Invalid".into(),
            employee_percent: "0".into(),
            employer_percent: "0".into(),
            employee_fixed: "-50".into(),
            employer_fixed: "0".into(),
            cap: None,
            ..Default::default()
        }],
        manual_tax: "0".into(),
        ..Default::default()
    };
    assert!(payroll(&e, "2026-11").is_err());
}
#[test]
fn daily_partial_absence_and_capped_split() {
    let e = Employee {
        id: "test".into(),
        name: "Test".into(),
        role: "".into(),
        structure: "Daily".into(),
        rate: "800".into(),
        standard_hours: "8".into(),
        overtime_multiplier: "1.5".into(),
        days: vec![
            Day {
                date: "2026-11-01".into(),
                status: "Present".into(),
                hours: "4".into(),
                overtime: "2".into(),
                extra: "100".into(),
                deduction: "25".into(),
                note: "Half day plus overtime".into(),
            },
            Day {
                date: "2026-11-02".into(),
                status: "Absent".into(),
                hours: "8".into(),
                overtime: "0".into(),
                extra: "0".into(),
                deduction: "0".into(),
                note: "No paid hours".into(),
            },
        ],
        contributions: vec![Contribution {
            name: "Custom verified-by-user scheme".into(),
            employee_percent: "1".into(),
            employer_percent: "2".into(),
            employee_fixed: "5".into(),
            employer_fixed: "0".into(),
            cap: Some("500".into()),
            ..Default::default()
        }],
        manual_tax: "50".into(),
        ..Default::default()
    };
    let p = payroll(&e, "2026-11").unwrap();
    assert_eq!(money(&p.gross).unwrap(), money("800").unwrap());
    assert_eq!(money(&p.net).unwrap(), money("715").unwrap());
    assert_eq!(money(&p.employer_cost).unwrap(), money("810").unwrap());
}
#[test]
fn duplicate_overpayment_and_currency_rejected() {
    let mut r = rec("2026-11", "Income", "100");
    let p = pay("80", "2026-11-05");
    r.payments.push(p.clone());
    r.payments.push(p);
    assert!(balance(&r, "").is_err());
    r.payments.pop();
    r.payments.push(pay("30", "2026-11-06"));
    assert!(balance(&r, "").is_err());
    let a = Metadata {
        business: Some(business("2026-12")),
        ..Default::default()
    };
    let mut b = Metadata {
        business: Some(business("2026-11")),
        ..Default::default()
    };
    b.business.as_mut().unwrap().currency = "USD".into();
    assert!(merge(&a, &b, true).is_err());
}
#[test]
fn late_payment_and_reversal_leave_old_revenue_unchanged() {
    let mut m = Metadata {
        business: Some(business("2026-12")),
        ..Default::default()
    };
    let mut invoice = rec("2026-10", "Income", "10000");
    invoice.payments.push(pay("4000", "2026-10-20"));
    let late = pay("2000", "2026-11-10");
    invoice.payments.push(late.clone());
    let mut reversal = pay("-2000", "2026-12-01");
    reversal.reversal_of = Some(late.id);
    invoice.payments.push(reversal);
    m.records.push(invoice);
    let charge = rec("2026-11", "Income", "100");
    m.records.push(charge);
    let s = summary(&m, "2026-11", "2026-11").unwrap();
    assert_eq!(s["revenue"], "100");
    assert_eq!(s["cash_received"], "2000");
    assert_eq!(s["receivables"], "4100");
    let s = summary(&m, "2026-12", "2026-12").unwrap();
    assert_eq!(s["revenue"], "0");
    assert_eq!(s["cash_received"], "-2000");
    assert_eq!(s["receivables"], "6100");
}
