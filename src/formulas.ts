export const functions = [
  {
    name: "SUM",
    description: "Add numbers or total expenses",
    inputs: "One or more cells or ranges",
    example: "SUM(A1:A3)",
    result: "60 when the cells contain 10, 20 and 30",
  },
  {
    name: "AVERAGE",
    description: "Find the average salary or typical amount",
    inputs: "One or more ranges",
    example: "AVERAGE(A1:A3)",
    result: "20 when the cells contain 10, 20 and 30",
  },
  {
    name: "MAX",
    description: "Find the highest amount or largest number",
    inputs: "One or more ranges",
    example: "MAX(A1:A3)",
    result: "30 when the cells contain 10, 20 and 30",
  },
  {
    name: "MIN",
    description: "Find the lowest amount or smallest number",
    inputs: "One or more ranges",
    example: "MIN(A1:A3)",
    result: "10 when the cells contain 10, 20 and 30",
  },
  {
    name: "COUNT",
    description: "Count how many numeric cells",
    inputs: "One or more ranges",
    example: "COUNT(A1:A3)",
    result: "3 when all three cells contain numbers",
  },
  {
    name: "COUNTA",
    description: "Count how many filled cells including text",
    inputs: "One or more ranges",
    example: "COUNTA(A1:A3)",
    result: "3 when all three cells are filled",
  },
  {
    name: "IF",
    description: "Calculate if a condition is true or false",
    inputs: "Condition, value if true, value if false",
    example: 'IF(A1>100,"High","Low")',
    result: "High if A1 is greater than 100",
  },
  {
    name: "SUMIF",
    description: "Total matching category",
    inputs: "Criteria range, condition, sum range",
    example: 'SUMIF(A1:A10,"Rent",B1:B10)',
    result: "Sum of amounts labelled Rent",
  },
  {
    name: "SUMIFS",
    description: "Total matching multiple conditions",
    inputs: "Sum range, criteria range, condition",
    example: 'SUMIFS(C1:C10,A1:A10,"Rent")',
    result: "Sum of matching amounts",
  },
  {
    name: "XLOOKUP",
    description: "Find matching value in another list",
    inputs: "Search value, lookup range, return range",
    example: "XLOOKUP(A1,B1:B10,C1:C10)",
    result: "Corresponding value from column C",
  },
  {
    name: "ROUND",
    description: "Round a number to decimal places",
    inputs: "Number, decimal places",
    example: "ROUND(12.345,2)",
    result: "12.35",
  },
  {
    name: "PMT",
    description: "Calculate a loan repayment",
    inputs: "Periodic interest rate, number of payments, principal",
    example: "PMT(0.01,12,10000)",
    result: "Periodic payment as a negative cash flow",
  },
  {
    name: "TODAY",
    description: "Show today’s date",
    inputs: "No inputs",
    example: "TODAY()",
    result: "Current local calendar date",
  },
  {
    name: "CONCAT",
    description: "Join text together",
    inputs: "Text or cell references",
    example: 'CONCAT(A1," ",B1)',
    result: "Joined text with a space",
  },
  {
    name: "Percentage",
    description: "Calculate a percentage part of an amount",
    inputs: "Amount cell multiplied by percentage",
    example: "A1*15%",
    result: "15 when A1 contains 100",
  },
];
export function col(n: number) {
  let s = "";
  while (n > 0) {
    n--;
    s = String.fromCharCode(65 + (n % 26)) + s;
    n = Math.floor(n / 26);
  }
  return s;
}
export function address(text: string) {
  const m = /^\$?([A-Z]{1,3})\$?([1-9]\d*)$/i.exec(text.trim());
  if (!m) throw Error("Use a cell address such as A1.");
  let c = 0;
  for (const a of m[1].toUpperCase()) c = c * 26 + a.charCodeAt(0) - 64;
  const r = Number(m[2]);
  if (r > 1048576 || c > 16384) throw Error("Cell is outside XLSX limits.");
  return { r, c };
}
