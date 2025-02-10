import axios from "axios";
import { DCAPAL_API, supabase } from "@app/config";

// Create `axios` instance passing the newly created `cache.adapter`
export const api = axios.create();

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
        fees: p.fees
          ? {
              feeStructure: {
                type: feeTypeToString(p.fees.feeStructure.type).toLowerCase(),
                // TODO: Add other feeStructure properties
              },
            }
          : null,
        assets: Object.entries(p.assets).map(([_, a]) => ({
          symbol: a.symbol.toLowerCase(),
          name: a.name,
          aclass: aclassToString(a.aclass),
          baseCcy: a.baseCcy,
          provider: a.provider,
          price: a.price,
          qty: a.qty,
          targetWeight: a.targetWeight,
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
    return response.json(); // Returns { updatedPortfolios: [], deletedPortfolios: [] }
  } catch (error) {
    console.error("Sync error:", error);
    return null;
  }
};
