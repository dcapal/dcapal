/**
 * @typedef {Record<string, Object>} AnalyzeRequest
 */

/**
 * @typedef {Object} SolveRequest
 * @property {number} budget
 * @property {Record<string, Object>} assets
 * @property {string} pfolioCcy
 * @property {Object|null|undefined} fees
 * @property {boolean} isBuyOnly
 * @property {boolean} useAllBudget
 */

/**
 * @typedef {Object|null} ComputeSolution
 */

const isPlainObject = (value) =>
  value !== null && typeof value === "object" && !Array.isArray(value);

const hasOwn = (obj, key) => Object.prototype.hasOwnProperty.call(obj, key);

/**
 * @param {unknown} assets
 * @returns {boolean}
 */
export const isAnalyzeRequest = (assets) => isPlainObject(assets);

/**
 * @param {unknown} request
 * @returns {boolean}
 */
export const isSolveRequest = (request) => {
  if (!isPlainObject(request)) return false;

  return (
    typeof request.budget === "number" &&
    isPlainObject(request.assets) &&
    typeof request.pfolioCcy === "string" &&
    hasOwn(request, "fees") &&
    typeof request.isBuyOnly === "boolean" &&
    typeof request.useAllBudget === "boolean"
  );
};
