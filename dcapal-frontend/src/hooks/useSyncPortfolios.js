import { useEffect, useCallback, useState } from "react";
import { usePortfolioStore } from "@/state/portfolioStore";
import { supabase } from "@app/config";

export function useSyncPortfolios(intervalMs = 5000) {
  const syncPortfoliosNow = usePortfolioStore(
    (state) => state.syncPortfoliosNow
  );
  const [isAuthenticated, setIsAuthenticated] = useState(false);

  // Check auth state on mount and listen for changes
  useEffect(() => {
    // Initial auth check
    supabase.auth.getSession().then(({ data: { session } }) => {
      setIsAuthenticated(!!session);
    });

    // Listen for auth changes
    const {
      data: { subscription },
    } = supabase.auth.onAuthStateChange((event, session) => {
      setIsAuthenticated(!!session);
    });

    return () => {
      subscription.unsubscribe();
    };
  }, []);

  const syncData = useCallback(() => {
    if (!isAuthenticated) return;
    syncPortfoliosNow();
  }, [syncPortfoliosNow, isAuthenticated]);

  useEffect(() => {
    if (!isAuthenticated) return;

    // Immediate sync on mount if authenticated
    syncData();

    // Set up the interval for subsequent syncs
    const intervalId = setInterval(syncData, intervalMs);

    return () => clearInterval(intervalId);
  }, [syncData, intervalMs, isAuthenticated]);

  // Expose the sync function and auth state for manual triggers
  return {
    syncNow: syncData,
    isAuthenticated,
  };
}
