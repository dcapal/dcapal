import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import { useDispatch, useSelector } from "react-redux";
import { useSyncPortfolios as useSyncPortfoliosMutation } from "@dcapal/api-client";

import { supabase } from "@app/config";
import { applySyncResult } from "@components/allocationFlow/portfolioSlice";
import { toSyncPayload } from "@/api/portfolioSync";

const SyncContext = createContext(null);

export const SyncCoordinator = ({ children, intervalMs = 5000 }) => {
  const dispatch = useDispatch();
  const pfolios = useSelector((state) => state.pfolio.pfolios);
  const deletedPortfolios = useSelector(
    (state) => state.pfolio.deletedPortfolios
  );
  const [isAuthenticated, setIsAuthenticated] = useState(false);

  const { mutateAsync, isPending } = useSyncPortfoliosMutation({
    mutation: {
      onSuccess: (response) => dispatch(applySyncResult(response.data)),
    },
  });
  const stateRef = useRef({ pfolios, deletedPortfolios });
  const pendingRef = useRef(isPending);
  stateRef.current = { pfolios, deletedPortfolios };
  pendingRef.current = isPending;

  useEffect(() => {
    let mounted = true;

    supabase.auth.getSession().then(({ data: { session } }) => {
      if (mounted) setIsAuthenticated(Boolean(session));
    });

    const {
      data: { subscription },
    } = supabase.auth.onAuthStateChange((_event, session) => {
      setIsAuthenticated(Boolean(session));
    });

    return () => {
      mounted = false;
      subscription.unsubscribe();
    };
  }, []);

  const syncNow = useCallback(() => {
    if (!isAuthenticated || pendingRef.current) return Promise.resolve();

    return mutateAsync({
      data: toSyncPayload(
        stateRef.current.pfolios,
        stateRef.current.deletedPortfolios
      ),
    }).catch((error) => {
      console.error("Sync error:", error);
    });
  }, [isAuthenticated, mutateAsync]);

  useEffect(() => {
    if (!isAuthenticated) return undefined;

    syncNow();
    const intervalId = setInterval(syncNow, intervalMs);
    return () => clearInterval(intervalId);
  }, [intervalMs, isAuthenticated, syncNow]);

  return (
    <SyncContext.Provider
      value={{
        isAuthenticated,
        isSyncing: isPending,
        syncNow,
      }}
    >
      {children}
    </SyncContext.Provider>
  );
};

export const useSyncPortfolios = () => {
  const context = useContext(SyncContext);
  if (!context) {
    throw new Error("useSyncPortfolios must be used inside SyncCoordinator");
  }

  return context;
};
