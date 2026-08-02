import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import type { ReactNode } from "react";
import { useSyncPortfolios as useSyncPortfoliosMutation } from "@dcapal/api-client";

import { supabase } from "@app/config";
import { toSyncPayload } from "@/api/portfolioSync";
import {
  applySyncPortfoliosResult,
  usePortfolioStore,
} from "@/state/portfolioStore";

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

/** Provides authenticated portfolio synchronization to the route tree. */
export const SyncCoordinator = ({
  children,
  intervalMs = 5000,
}: SyncCoordinatorProps) => {
  const pfolios = usePortfolioStore((state) => state.pfolios);
  const deletedPortfolios = usePortfolioStore(
    (state) => state.deletedPortfolios
  );
  const [isAuthenticated, setIsAuthenticated] = useState(false);

  const { mutateAsync, isPending } = useSyncPortfoliosMutation({
    mutation: {
      onSuccess: (response) => {
        usePortfolioStore.setState((state) =>
          applySyncPortfoliosResult(state, response.data)
        );
      },
    },
  });
  // The interval callback must read the latest store state without being
  // recreated every five seconds.
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

/** Reads the synchronization state supplied by `SyncCoordinator`. */
export const useSyncPortfolios = () => {
  const context = useContext(SyncContext);
  if (!context) {
    throw new Error("useSyncPortfolios must be used inside SyncCoordinator");
  }

  return context;
};
