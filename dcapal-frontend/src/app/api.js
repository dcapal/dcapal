import axios from "axios";
import { supabase } from "@app/config";

// Create `axios` instance passing the newly created `cache.adapter`
export const api = axios.create();

export const setAuthToken = async () => {
  const {
    data: { session },
  } = await supabase.auth.getSession();
  if (session) {
    api.defaults.headers.common["Authorization"] =
      `Bearer ${session.access_token}`;
  } else {
    delete api.defaults.headers.common["Authorization"];
  }
};

export const syncPortfolios = async (localPortfolios) => {
  const response = await api.post("/sync/portfolios", {
    portfolios: localPortfolios,
  });
  return response.data;
};

export const fetchPortfolios = async () => {
  const response = await api.get("/portfolios");
  return response.data;
};
