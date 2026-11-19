use crate::{custom, live, Document};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const STANDARD: &[&str] = &["Customer / recipient", "Description", "Amount", "Tax", "Deductible", "Status", "Due date"];
const REPAIRS: &[&str] = &["Repair shop", "Vehicle registration", "Total paid / invoiced", "Tax", "Deductible", "Paid", "Outstanding"];

pub fn transactions(doc: &Document) -> Result<Value, String> {
    let mut tables = BTreeMap::new();
    for (category, sheet) in [("Income", "Income"), ("Expense", "Expenses"), ("Asset", "Assets"), ("Refund", "Refunds"), ("Vehicle repairs", "Vehicle Repairs")] {
        let Some(index) = custom::index(&doc.model, sheet) else { continue };
        let repair = category == "Vehicle repairs";
        let wanted = if repair { REPAIRS } else { STANDARD };
        let width = doc.model.get_model().workbook.worksheets[index as usize].dimension().max_column;
        let mut sheet_columns = BTreeMap::new();
        for column in 1..=width {
            sheet_columns.insert(doc.model.get_formatted_cell_value(index, 3, column)?, column);
        }
        let headers: Vec<_> = wanted.iter().copied().filter(|label| {
            if repair && matches!(*label, "Tax" | "Deductible") {
                let detail = if *label == "Tax" { "tax included" } else { "deductible" };
                return ["Part", "Labour"].iter().any(|part| sheet_columns.contains_key(&format!("{part} {detail}")));
            }
            sheet_columns.contains_key(*label)
        }).collect();
        let mut rows = BTreeMap::new();
        let records = doc.meta.records.iter().filter(|r| r.category == category || (category == "Expense" && r.category == "Other payable"));
        for (offset, record) in records.enumerate() {
            let row = offset as i32 + 4;
            let mut cells = Vec::with_capacity(headers.len());
            for label in &headers {
                let value = if let Some(column) = sheet_columns.get(*label) {
                    doc.model.get_formatted_cell_value(index, row, *column)?
                } else {
                    let (tax, deductible) = live::transaction_tax(record)?;
                    if *label == "Tax" { format!("{tax:.2}") } else { format!("{deductible:.2}") }
                };
                cells.push(value);
            }
            rows.insert(&record.id, cells);
        }
        tables.insert(sheet, json!({"headers":headers,"rows":rows}));
    }
    Ok(json!(tables))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{finance::{Business, Record, TransactionTax, Vehicle}, templates};

    #[test]
    fn repairs_show_one_total_for_each_calculation() {
        let business = Business { company_id:"fleet".into(), name:"Fleet".into(), period:"2026-11".into(),
            currency:"ZAR".into(), country:"South Africa".into(), year_start:1, ..Default::default() };
        let (mut model, mut meta) = templates::create("General Business", Some(business)).unwrap();
        let calculation = TransactionTax { tax_percent:"10".into(), tax_fixed:"2".into(),
            deductible_fixed:"3".into(), ..Default::default() };
        meta.records.push(Record { id:"repair".into(), revision:1, period:"2026-11".into(),
            category:"Vehicle repairs".into(), amount:"150".into(), vehicle:Vehicle {
                part_amount:"100".into(), labour_amount:"50".into(),
                part:calculation.clone(), labour:calculation, ..Default::default()
            }, ..Default::default() });
        templates::sync(&mut model, &meta).unwrap();
        let doc = Document {model, meta, path:None, dirty:true};
        let tables = transactions(&doc).unwrap();
        let table = &tables["Vehicle Repairs"];
        assert_eq!(table["headers"], json!(REPAIRS));
        for (label, expected) in [("Tax","19.00"), ("Deductible","6.00")] {
            let column = REPAIRS.iter().position(|h| *h == label).unwrap();
            assert_eq!(table["rows"]["repair"][column], expected);
        }
    }
}
