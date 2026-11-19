use crate::{finance::*,live,templates,files};
use rust_decimal::Decimal;
fn meta()->Metadata {Metadata{business:Some(Business{company_id:"v4-test".into(),name:"V4 Test".into(),currency:"ZAR".into(),country:"South Africa".into(),period:"2026-11".into(),year_start:3,..Default::default()}),template:"General Business".into(),..Default::default()}}
fn record(category:&str,amount:&str)->Record {Record{id:uuid::Uuid::new_v4().to_string(),revision:1,period:"2026-11".into(),category:category.into(),party:"Supplier".into(),description:"Synthetic".into(),amount:amount.into(),..Default::default()}}
fn rule()->TaxRule {TaxRule{id:"income-tax".into(),name:"Progressive tax".into(),automatic:true,bands:vec![TaxBand{lower:"0".into(),upper:"100000".into(),percent:"20".into(),fixed:String::new()},TaxBand{lower:"100000".into(),upper:String::new(),percent:"30".into(),fixed:String::new()}],..Default::default()}}
fn eq(a:&str,b:&str){assert_eq!(money(a).unwrap(),money(b).unwrap());}
#[test] fn v4_marginal_ranges_boundaries_and_fixed_do_not_double_count(){
 let mut t=rule();for (income,expected) in [("0","0"),("99999","19999.80"),("100000","20000"),("100000.01","20000"),("150000","35000"),("200000","50000"),("250000","65000")]{assert_eq!(live::rule_tax(&t,money(income).unwrap()).unwrap(),money(expected).unwrap());}
 t.bands[1].fixed="20000".into();assert_eq!(live::rule_tax(&t,money("200000").unwrap()).unwrap(),money("50000").unwrap());
 t.bands[1].lower="99999".into();assert!(live::rule_tax(&t,Decimal::ZERO).is_err());
 t.bands[1].lower="100001".into();assert!(live::rule_tax(&t,Decimal::ZERO).is_err());
 t.bands[1].lower="100000".into();t.bands[1].upper="200000".into();assert!(live::rule_tax(&t,Decimal::ZERO).is_err());
}
#[test] fn v4_deduction_bases_and_targets_are_independent(){
 let mut r=record("Expense","1000");r.calculation=TransactionTax{tax_percent:"10".into(),tax_fixed:"20".into(),deductible_percent:"50".into(),deductible_fixed:"5".into(),deductible_base:"Tax".into(),deduction_target:"Income".into(),purchase_tax:true};
 let (tax,d)=live::transaction_tax(&r).unwrap();assert_eq!(tax,money("120").unwrap());assert_eq!(d,money("65").unwrap());assert_eq!(live::transaction_deductions(&r).unwrap(),(d,Decimal::ZERO));
 r.calculation.deductible_base="Total".into();r.calculation.deduction_target="Tax".into();assert_eq!(live::transaction_deductions(&r).unwrap(),(Decimal::ZERO,money("505").unwrap()));assert_eq!(live::transaction_liability(&r).unwrap(),Decimal::ZERO);
 r.calculation.deductible_percent="0".into();eq(&live::transaction_tax(&r).unwrap().1.to_string(),"5");
}
#[test] fn v4_income_deductions_credits_and_live_obligations(){
 let mut m=meta();m.business_taxes.push(rule());m.records.push(record("Income","210000"));let mut r=record("Expense","10000");r.calculation.deductible_fixed="10000".into();r.calculation.deduction_target="Income".into();m.records.push(r);
 m.deductions.push(ManualDeduction{id:"manual".into(),name:"Unlisted deduction".into(),percent:"0".into(),fixed:"10000".into(),target:"Income".into()});
 m.deductions.push(ManualDeduction{id:"credit".into(),name:"Credit".into(),percent:"0".into(),fixed:"500".into(),target:"Tax".into()});
 live::sync_records(&mut m).unwrap();let v=live::tax_view(&m).unwrap();eq(&v.taxable_income,"190000");eq(&v.income_deductions,"20000");eq(&v.employer_tax,"47000");eq(&v.net_employer_tax,"46500");
 let count=m.records.len();live::sync_records(&mut m).unwrap();assert_eq!(count,m.records.len());eq(&m.records.iter().find(|r|r.managed_key.ends_with("employer-rule:income-tax")).unwrap().amount,"46500");
 m.records.iter_mut().find(|r|r.category=="Income").unwrap().amount="220000".into();live::sync_records(&mut m).unwrap();eq(&live::tax_view(&m).unwrap().net_employer_tax,"49500");
 m.deductions[0].fixed="300000".into();live::sync_records(&mut m).unwrap();let v=live::tax_view(&m).unwrap();eq(&v.taxable_income,"0");eq(&v.net_employer_tax,"0");eq(&v.unused_deductions,"500");
}
fn vehicle_meta()->Metadata{
 let mut m=meta();m.business_taxes.push(rule());m.records.push(record("Income","200000"));
 let mut fuel=record("Fuel","1000");fuel.invoice="INV-F001".into();fuel.vehicle=Vehicle{registration:"TEST 001".into(),employee:"Ada".into(),price_per_litre:"25".into(),litres:"40".into(),..Default::default()};fuel.calculation=TransactionTax{tax_percent:"10".into(),deductible_percent:"50".into(),deductible_fixed:"10".into(),deductible_base:"Tax".into(),deduction_target:"Tax".into(),purchase_tax:true,..Default::default()};m.records.push(fuel);
 let mut repair=record("Vehicle repairs","3000");repair.invoice="INV-R001".into();repair.vehicle=Vehicle{registration:"TEST 001".into(),employee:"Ada".into(),part_name:"Brake pads".into(),part_number:"BP-01".into(),part_amount:"2000".into(),labour_amount:"1000".into(),part:TransactionTax{tax_percent:"10".into(),deductible_percent:"25".into(),deductible_fixed:"20".into(),deduction_target:"Income".into(),purchase_tax:true,..Default::default()},labour:TransactionTax{tax_fixed:"100".into(),deductible_percent:"50".into(),deductible_base:"Tax".into(),deduction_target:"Tax".into(),purchase_tax:true,..Default::default()},..Default::default()};m.records.push(repair);m
}
#[test] fn v4_vehicle_costs_purchase_tax_and_separate_deductions(){
 let mut m=vehicle_meta();let repair=m.records.last().unwrap();let v=live::transaction_tax(repair).unwrap();eq(&v.0.to_string(),"300");eq(&v.1.to_string(),"570");
 live::sync_records(&mut m).unwrap();let v=live::tax_view(&m).unwrap();eq(&v.taxable_income,"199480");eq(&v.employer_tax,"49844");eq(&v.deductions,"110");eq(&v.net_employer_tax,"49734");
 eq(summary(&m,"2026-11","2026-11").unwrap()["expenses"].as_str().unwrap(),"53734");
 m.records.iter_mut().find(|r|r.category=="Vehicle repairs").unwrap().amount="2999".into();assert!(validate(&m).is_err());
}
#[test] fn v4_employee_identity_shares_extras_and_rollover(){
 let mut m=meta();let e=Employee{id:"employee1".into(),number:"E001".into(),site:"North".into(),name:"Ada".into(),structure:"Monthly".into(),rate:"10000".into(),standard_hours:"8".into(),overtime_multiplier:"1.5".into(),tax_percent:"10".into(),manual_tax:"0".into(),contributions:vec![Contribution{name:"Pension".into(),employee_percent:"40".into(),employer_percent:"60".into(),total_percent:"10".into(),total_fixed:"0".into(),employee_fixed:"0".into(),employer_fixed:"0".into(),split_pool:true,..Default::default()}],..Default::default()};m.employees.push(e);
 m.employee_extras.push(EmployeeExtra{id:"extra".into(),number:"E002".into(),employee:"Ben".into(),name:"External contribution".into(),kind:"Contribution".into(),base:"100".into(),percent:"10".into(),fixed:"5".into()});
 live::sync_records(&mut m).unwrap();let v=live::tax_view(&m).unwrap();eq(&v.employee_tax,"1000");eq(&v.employee_contributions,"415");eq(&v.employer_contributions,"600");eq(&payroll(&m.employees[0],"2026-11").unwrap().net,"8600");assert!(v.rows.iter().any(|r|r.source=="E001 · Ada"));assert!(v.rows.iter().any(|r|r.source=="E002 · Ben"));
 let mut b=m.business.clone().unwrap();b.period="2026-12".into();let next=live::rollover(&m,b).unwrap();assert_eq!(next.employees[0].number,"E001");assert_eq!(next.employees[0].site,"North");assert!(next.employee_extras.is_empty());
 m.employee_extras[0].number.clear();assert!(validate(&m).is_err());
}
#[test] fn v4_worksheet_and_file_roundtrips_preserve_all_fields(){
 let mut m=vehicle_meta();live::sync_records(&mut m).unwrap();let (mut model,_)=templates::create("General Business",m.business.clone()).unwrap();templates::sync(&mut model,&m).unwrap();
 assert_eq!(model.get_worksheets_properties().len(),17);
 assert_eq!(model.get_formatted_cell_value(14,4,2).unwrap(),"Supplier");
 assert_eq!(model.get_formatted_cell_value(14,4,16).unwrap(),"INV-F001");
 assert_eq!(model.get_formatted_cell_value(15,4,6).unwrap(),"BP-01");
 assert_eq!(model.get_formatted_cell_value(15,4,26).unwrap(),"INV-R001");
 let dir=tempfile::tempdir().unwrap();for ext in ["xlsx","ods"]{let file=dir.path().join(format!("v4.{ext}"));files::save(file.to_str().unwrap(),&model,&m).unwrap();let (_,loaded)=files::load(file.to_str().unwrap()).unwrap();assert_eq!(loaded.schema,4);assert_eq!(loaded.records[1].invoice,"INV-F001");assert_eq!(loaded.records[2].vehicle.part_number,"BP-01");assert_eq!(loaded.business_taxes[0].bands.len(),2);eq(&live::tax_view(&loaded).unwrap().net_employer_tax,"49734");}
}

