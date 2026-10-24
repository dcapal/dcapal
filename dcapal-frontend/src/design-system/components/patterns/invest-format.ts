/** Formats a quantity of units, dropping decimals when the quantity is whole. */
export function formatQuantity(value: number): string {
  return new Intl.NumberFormat("en-IE", {
    minimumFractionDigits: 0,
    maximumFractionDigits: 4,
  }).format(value);
}

/** Formats a quantity change with an explicit sign, using a true minus sign. */
export function formatQuantityChange(value: number): string {
  if (value > 0) return `+${formatQuantity(value)}`;
  if (value < 0) return `−${formatQuantity(Math.abs(value))}`;
  return formatQuantity(0);
}

/** Formats a distance in percentage points without a sign, such as `29.5 pp`. */
export function formatPoints(value: number): string {
  return `${value.toFixed(1)} pp`;
}

/** Formats a signed deviation in percentage points with a true minus sign. */
export function formatDeviation(value: number): string {
  const rounded = value.toFixed(1);
  if (value > 0 && rounded !== "0.0") return `+${rounded} pp`;
  if (value < 0 && rounded !== "-0.0") {
    return `−${Math.abs(value).toFixed(1)} pp`;
  }
  return "0.0 pp";
}

/** Formats a fee impact as a percentage with two decimals, such as `0.67%`. */
export function formatFeeImpact(value: number): string {
  return `${value.toFixed(2)}%`;
}
