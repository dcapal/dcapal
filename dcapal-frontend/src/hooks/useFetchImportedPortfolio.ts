import { useGetImportedPortfolio } from "@dcapal/api-client";

export const useFetchImportedPortfolio = (portfolioId?: string | null) => {
  const query = useGetImportedPortfolio(portfolioId || "", {
    query: {
      enabled: Boolean(portfolioId),
    },
  });

  return {
    portfolio: query.data?.data ?? null,
    isLoading: Boolean(portfolioId) && query.isPending,
    isError: query.isError,
  };
};
