export const categories = ["Personal & household", "School & education", "Projects & work", "Business"];
const personal = new Set(["Monthly Personal Budget","Monthly Budget Summary","Annual Budget","Annual Budget Summary","Household Expenses","Savings Tracker","Debt Repayment Tracker","Personal Income Tracker"]);
const school = new Set(["Gradebook","Attendance Register","Assignment Tracker","Student Performance Report","School Budget","School Budget Summary"]);
const projects = new Set(["Project Budget","Project Budget Summary","Project Planner","Work Schedule"]);

export function templateCategory(name: string) {
  if (personal.has(name)) return categories[0];
  if (school.has(name)) return categories[1];
  if (projects.has(name)) return categories[2];
  return categories[3];
}
