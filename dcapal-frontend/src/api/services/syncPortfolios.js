import { DCAPAL_API, supabase } from "@app/config";
import {
  aclassToString,
  FeeType,
  feeTypeToString,
} from "@components/allocationFlow/portfolioSlice";

const parseFees = (fees) => {
  if (!fees) return null;
  const feeStructure = {
    type: feeTypeToString(fees.feeStructure.type),
    maxFeeImpact: fees.feeStructure.maxFeeImpact,
  };

  switch (fees.feeStructure.type) {
    case FeeType.FIXED:
      feeStructure.feeAmount = fees.feeStructure.feeAmount;
      break;
    case FeeType.VARIABLE:
      feeStructure.feeRate = fees.feeStructure.feeRate;
      feeStructure.minFee = fees.feeStructure.minFee;
      if (fees.feeStructure.maxFee) {
        feeStructure.maxFee = fees.feeStructure.maxFee;
      }
      break;
    case FeeType.ZERO_FEE:
    default:
      break;
  }

  return { feeStructure };
};

export const syncPortfoliosAPI = async (portfolios, deletedPortfolios) => {
  try {
    const {
      data: { session },
    } = await supabase.auth.getSession();
    if (!session) throw new Error("Not authenticated");

    const transformedData = {
      portfolios: Object.values(portfolios).map((p) => ({
        id: p.id,
        name: p.name,
        quoteCcy: p.quoteCcy,
        fees: p.fees ? parseFees(p.fees) : null,
        assets: Object.entries(p.assets).map(([_, a]) => ({
          symbol: a.symbol.toLowerCase(),
          name: a.name,
          aclass: aclassToString(a.aclass),
          baseCcy: a.baseCcy,
          provider: a.provider,
          price: a.price,
          qty: a.qty,
          targetWeight: a.targetWeight,
          averageBuyPrice: a.averageBuyPrice ?? a.price,
          fees: parseFees(a.fees),
        })),
        lastUpdatedAt: new Date(p.lastUpdatedAt).toISOString(),
      })),
      deletedPortfolios,
    };

    const url = `${DCAPAL_API}/v1/sync/portfolios`;
    const response = await fetch(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${session.access_token}`,
      },
      body: JSON.stringify(transformedData),
    });

    if (!response.ok) throw new Error("Sync failed");
    return response.json();
  } catch (error) {
    console.error("Sync error:", error);
    return null;
  }
};
