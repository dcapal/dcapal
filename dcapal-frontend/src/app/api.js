import axios from "axios";
import {DCAPAL_API, supabase} from "@app/config";

// Create `axios` instance passing the newly created `cache.adapter`
export const api = axios.create();

export const syncPortfoliosAPI = async (portfolios, deletedPortfolios) => {
  try {
    const { data: { session } } = await supabase.auth.getSession();
    if (!session) throw new Error("Not authenticated");

    const url = `${DCAPAL_API}/v1/sync/portfolios`;

    const response = await fetch(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${session.access_token}`,
      },
      body: JSON.stringify({ portfolios, deletedPortfolios }),
    });

    if (!response.ok) throw new Error("Sync failed");
    return response.json(); // Returns { updatedPortfolios: [], deletedPortfolios: [] }
  } catch (error) {
    console.error("Sync error:", error);
    return null;
  }
};
