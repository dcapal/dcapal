import { useEffect, useState } from "react";

import { fetchImportedPortfolio } from "@/api";

export const useFetchImportedPortfolio = (portfolioId) => {
  const [portfolio, setPortfolio] = useState(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    if (!portfolioId) {
      setPortfolio(null);
      setIsLoading(false);
      return;
    }

    let ignore = false;
    fetchPortfolio(portfolioId, setPortfolio, setIsLoading, ignore);

    return () => {
      ignore = true;
    };
  }, [portfolioId]);

  return [portfolio, isLoading];
};

const fetchPortfolio = async (
  portfolioId,
  setPortfolio,
  setIsLoading,
  ignore
) => {
  const p = await fetchImportedPortfolio(portfolioId);
  if (p && !ignore) {
    setPortfolio(p);
    setIsLoading(false);
  } else {
    setPortfolio(null);
    setIsLoading(false);
  }
};
