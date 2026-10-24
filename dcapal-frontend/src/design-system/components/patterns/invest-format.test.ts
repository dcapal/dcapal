import { describe, expect, it } from "vitest";
import {
  formatDeviation,
  formatPoints,
  formatTargetWeight,
  formatQuantity,
  formatQuantityChange,
} from "./invest-format";

describe("invest formatting", () => {
  /**
   * GIVEN whole and fractional quantities
   * WHEN they are formatted
   * THEN whole quantities have no decimals and fractions keep up to four.
   */
  it("formats quantities", () => {
    expect(formatQuantity(149)).toBe("149");
    expect(formatQuantity(1.23456)).toBe("1.2346");
  });

  /**
   * GIVEN positive, negative and zero quantity changes
   * WHEN they are formatted
   * THEN the sign is always explicit except for zero.
   */
  it("formats quantity changes with a sign", () => {
    expect(formatQuantityChange(7)).toBe("+7");
    expect(formatQuantityChange(-2)).toBe("−2");
    expect(formatQuantityChange(0)).toBe("0");
  });

  /**
   * GIVEN deviations in percentage points
   * WHEN they are formatted
   * THEN positive values get a plus, negative a minus and near-zero none.
   */
  it("formats deviations", () => {
    expect(formatDeviation(25.54)).toBe("+25.5 pp");
    expect(formatDeviation(-23.1)).toBe("−23.1 pp");
    expect(formatDeviation(0.01)).toBe("0.0 pp");
    expect(formatPoints(29.5)).toBe("29.5 pp");
  });

  /**
   * GIVEN whole and fractional target weights
   * WHEN they are formatted as target weights
   * THEN whole weights drop the decimal and others keep one.
   */
  it("formats target weights", () => {
    expect(formatTargetWeight(65)).toBe("65%");
    expect(formatTargetWeight(12.5)).toBe("12.5%");
  });
});
