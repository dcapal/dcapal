export interface AnalyzeAsset {
  symbol?: string;
  target_weight?: number;
  shares?: number;
  price?: number;
  is_whole_shares?: boolean;
  current_amount?: number;
}

export type AnalyzeRequest = Record<string, AnalyzeAsset>;

export interface FeeStructureInput {
  type: string;
  feeAmount?: number;
  feeRate?: number;
  minFee?: number;
  maxFee?: number | null;
}

export interface TransactionFeesInput {
  maxFeeImpact?: number | null;
  feeStructure: FeeStructureInput;
}

export interface SolveAsset {
  symbol?: string;
  shares?: number;
  price?: number;
  target_weight?: number;
  is_whole_shares?: boolean;
  fees?: TransactionFeesInput | null;
}

export type SolveAssets = Record<string, SolveAsset>;

export interface SolveRequest {
  budget: number;
  assets: SolveAssets;
  pfolioCcy: string;
  fees: TransactionFeesInput | null | undefined;
  isBuyOnly: boolean;
  useAllBudget: boolean;
}

export interface ComputeSolveSolution {
  amounts?: Map<string, number>;
  shares?: Map<string, number>;
  theo_allocs?: Map<string, number>;
  budget_left?: number;
}

export type ComputeSolution = number | ComputeSolveSolution | null;

const isPlainObject = (value: unknown): value is Record<string, unknown> =>
  value !== null && typeof value === "object" && !Array.isArray(value);

const hasOwn = (obj: object, key: string): boolean =>
  Object.prototype.hasOwnProperty.call(obj, key);

export const isAnalyzeRequest = (assets: unknown): assets is AnalyzeRequest =>
  isPlainObject(assets);

export const isSolveRequest = (request: unknown): request is SolveRequest => {
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
