export const timeout = (ms) => {
  return new Promise((resolve) => setTimeout(resolve, ms));
};

export const roundAmount = (a) => Math.round(a * 100) / 100;
