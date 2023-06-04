export const timeout = (ms) => {
  return new Promise((resolve) => setTimeout(resolve, ms));
};

export const roundDecimals = (a, n) => {
  return Math.round((a + Number.EPSILON) * Math.pow(10, n)) / Math.pow(10, n);
};

export const roundAmount = (a) => roundDecimals(a, 2);
export const roundPrice = (a) => roundDecimals(a, 4);
