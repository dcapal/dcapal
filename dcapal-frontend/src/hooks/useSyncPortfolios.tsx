import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import type { ReactNode } from "react";
import { useDispatch, useSelector } from "react-redux";
import { useSyncPortfolios as useSyncPortfoliosMutation } from "@dcapal/api-client";

import { supabase } from "@app/config";
import { applySyncResult } from "@components/allocationFlow/portfolioSlice";
import { toSyncPayload } from "@/api/portfolioSync";

type SyncState = {
  pfolio: {
    pfolios: Parameters<typeof toSyncPayload>[0];
    deletedPortfolios: Parameters<typeof toSyncPayload>[1];
  };
};

type SyncContextValue = {
  isAuthenticated: boolean;
  isSyncing: boolean;
  syncNow: () => Promise<void>;
};

type SyncCoordinatorProps = {
  children: ReactNode;
  intervalMs?: number;
};

const SyncContext = createContext<SyncContextValue | null>(null);

export const SyncCoordinator = ({
  children,
  intervalMs = 5000,
}: SyncCoordinatorProps) => {
  const dispatch = useDispatch();
  const pfolios = useSelector((state: SyncState) => state.pfolio.pfolios);
  const deletedPortfolios = useSelector(
    (state: SyncState) => state.pfolio.deletedPortfolios
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

  const syncNow = useCallback(async (): Promise<void> => {
    if (!isAuthenticated || pendingRef.current) return;

    try {
      await mutateAsync({
        data: toSyncPayload(
          stateRef.current.pfolios,
          stateRef.current.deletedPortfolios
        ),
      });
    } catch (error) {
      console.error("Sync error:", error);
    }
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
