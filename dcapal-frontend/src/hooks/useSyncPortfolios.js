import { useEffect } from "react";
import { useDispatch } from "react-redux";
import { syncPortfolios } from "@components/allocationFlow/portfolioSlice";

export function useSyncPortfolios(intervalMs = 60000) {
  const dispatch = useDispatch();

  useEffect(() => {
    const intervalId = setInterval(() => {
      dispatch(syncPortfolios());
    }, intervalMs);

    return () => clearInterval(intervalId);
  }, [dispatch, intervalMs]);
}
