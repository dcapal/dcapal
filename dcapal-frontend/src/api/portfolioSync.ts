import type { SyncPortfoliosRequest } from "@dcapal/api-client";

import {
  aclassToString,
  feeTypeToString,
} from "@components/allocationFlow/portfolioSlice";

type FeeState = {
  feeStructure?: {
    type: number;
    feeAmount?: number | null;
    feeRate?: number | null;
    minFee?: number | null;
    maxFee?: number | null;
  } | null;
  maxFeeImpact?: number | null;
} | null;

type AssetState = {
  symbol: string;
  name: string;
  aclass: number;
  baseCcy: string;
  provider: string;
  price: number;
  qty: number;
  targetWeight: number;
  averageBuyPrice?: number | null;
  fees?: FeeState;
};

type PortfolioState = {
  id: string;
  name: string;
  quoteCcy: string;
  fees?: FeeState;
  assets: Record<string, AssetState>;
  lastUpdatedAt: number | string | Date;
};

const decimalString = (value: number | null | undefined): string =>
  String(value ?? 0);

const toFeesPayload = (
  fees: FeeState | undefined
): SyncPortfoliosRequest["portfolios"][number]["fees"] => {
  if (!fees?.feeStructure) return null;

  const { type, feeAmount, feeRate, minFee, maxFee } = fees.feeStructure;
  const feeType = feeTypeToString(type);
  const maxFeeImpact =
    fees.maxFeeImpact != null
      ? { maxFeeImpact: decimalString(fees.maxFeeImpact) }
      : {};

  if (feeType === "fixed") {
    return {
      feeStructure: {
        type: "fixed",
        feeAmount: decimalString(feeAmount),
      },
      ...maxFeeImpact,
    };
  }

  if (feeType === "variable") {
    return {
      feeStructure: {
        type: "variable",
        feeRate: decimalString(feeRate),
        minFee: decimalString(minFee),
        ...(maxFee != null ? { maxFee: decimalString(maxFee) } : {}),
      },
      ...maxFeeImpact,
    };
  }

  if (feeType === "zeroFee") {
    return {
      feeStructure: { type: "zeroFee" },
      ...maxFeeImpact,
    };
  }

  return null;
};

export const toSyncPayload = (
  pfolios: Record<string, PortfolioState>,
  deletedPortfolios: string[]
): SyncPortfoliosRequest => ({
  portfolios: Object.values(pfolios).map((portfolio) => ({
    id: portfolio.id,
    name: portfolio.name,
    quoteCcy: portfolio.quoteCcy,
    fees: toFeesPayload(portfolio.fees),
    assets: Object.values(portfolio.assets).map((asset) => ({
      symbol: asset.symbol.toLowerCase(),
      name: asset.name,
      aclass: aclassToString(asset.aclass),
      baseCcy: asset.baseCcy,
      provider: asset.provider,
      price: decimalString(asset.price),
      qty: decimalString(asset.qty),
      targetWeight: decimalString(asset.targetWeight),
      averageBuyPrice: decimalString(asset.averageBuyPrice ?? asset.price),
      fees: toFeesPayload(asset.fees),
    })),
    lastUpdatedAt: new Date(portfolio.lastUpdatedAt).toISOString(),
  })),
  deletedPortfolios,
});
