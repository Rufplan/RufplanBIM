/** A plant's size in feet (and inches under 3'), as landscape schedules give it. */
export function formatFeet(mm: number): string {
  const inches = mm / 25.4;
  if (inches < 36) return `${Math.round(inches)}"`;
  return `${Math.round(inches / 12)}'`;
}
