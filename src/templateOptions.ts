type Option = { name: string; columns: string[] };

const transactionSheets = new Set(["Income", "Expenses", "Assets", "Refunds", "Fuel", "Vehicle Repairs"]);

function optionName(sheet: string, column: string): string {
  if (transactionSheets.has(sheet)) {
    if (/\btax\b/i.test(column)) return "Tax";
    if (/\b(deductible|deduction)\b/i.test(column)) return "Deductibles";
    if (column.startsWith("Discount ")) return "Discount";
    if (["Paid","Outstanding","Due date","Status","Paid date"].includes(column)) return "Payment details";
  }
  if (sheet === "Fuel") {
    if (["Start odometer","End odometer"].includes(column)) return "Odometer readings";
    if (["Fuel quantity","Volume unit"].includes(column)) return "Fuel quantity";
    if (["Distance","Distance unit"].includes(column)) return "Distance travelled";
    if (["Fuel economy","Economy unit"].includes(column)) return "Fuel economy";
  }
  if (sheet === "Payroll" || sheet === "Payslips") {
    if (["Employee tax","Employer-paid tax","Tax"].includes(column)) return "Tax";
    if (["Employee contributions","Employer contributions","Individual contributions","Total contributions"].includes(column)) return "Contributions";
    if (["Salary paid","Salary remaining","Paid date","Payment status"].includes(column)) return "Salary payments";
  }
  if (sheet === "Outstanding Balances" && ["Paid","Remaining","Due date","Status","Paid date"].includes(column)) return "Payment details";
  return column;
}

export function templateOptions(sheet: string, columns: string[]): Option[] {
  const groups = new Map<string, Option>();
  for (const column of columns) {
    const name = optionName(sheet, column);
    if (!groups.has(name)) groups.set(name, { name, columns: [] });
    groups.get(name)!.columns.push(column);
  }
  return [...groups.values()];
}
