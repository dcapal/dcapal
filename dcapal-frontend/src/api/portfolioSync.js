import {
  aclassToString,
  FeeType,
  feeTypeToString,
} from "@components/allocationFlow/portfolioSlice";

const decimalString = (value) => String(value ?? 0);

const toFeesPayload = (fees) => {
  if (!fees?.feeStructure) return null;

  const { type, feeAmount, feeRate, minFee, maxFee, maxFeeImpact } = {
    ...fees.feeStructure,
    maxFeeImpact: fees.maxFeeImpact,
  };
  const feeType = feeTypeToString(type);
  const feeStructure = { type: feeType };

  if (type === FeeType.FIXED) {
    feeStructure.feeAmount = decimalString(feeAmount);
  }

  if (type === FeeType.VARIABLE) {
    feeStructure.feeRate = decimalString(feeRate);
    feeStructure.minFee = decimalString(minFee);
    if (maxFee != null) feeStructure.maxFee = decimalString(maxFee);
  }

  return {
    feeStructure,
    ...(maxFeeImpact != null
      ? { maxFeeImpact: decimalString(maxFeeImpact) }
      : {}),
  };
};

export const toSyncPayload = (pfolios, deletedPortfolios) => ({
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
