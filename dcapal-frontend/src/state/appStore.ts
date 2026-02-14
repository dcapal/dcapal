import { Step } from "@app/appSlice";
import { create } from "zustand";
import {
  createJSONStorage,
  persist,
  type StateStorage,
} from "zustand/middleware";

export type AppStoreState = {
  allocationFlowStep: number;
  currencies: string[];
  preferredCurrency: string;
  pfolioFile: string;
};

export type AppStoreActions = {
  setAllocationFlowStep: (payload: { step: number }) => void;
  setCurrencies: (payload: { currencies: string[] }) => void;
  setPreferredCurrency: (payload: { ccy: string }) => void;
  setPfolioFile: (payload: { file: string }) => void;
  hydrateFromRedux: (appState: Partial<AppStoreState> | undefined) => void;
};

export type AppStore = AppStoreState & AppStoreActions;

type ReduxStore = {
  getState: () => { app: AppStoreState };
  subscribe: (listener: () => void) => () => void;
};

type SyncStorage = {
  getItem: (name: string) => string | null;
  setItem: (name: string, value: string) => void;
  removeItem: (name: string) => void;
};

type LegacyPfolioState = {
  assets?: Record<string, unknown>;
  quoteCcy?: string;
};

type LegacyReduxPersistRootState = {
  app?: Partial<AppStoreState>;
  pfolio?: LegacyPfolioState;
  _persist?: {
    version?: unknown;
  };
};

type PersistedAppStoreState = Partial<AppStoreState> & {
  __legacyPfolio?: LegacyPfolioState;
};

export const APP_STORE_PERSIST_KEY = "zustand:appStore";
export const APP_STORE_PERSIST_VERSION = 5;

const REDUX_PERSIST_ROOT_KEY = "persist:root";

const inMemoryStorage = (() => {
  const map = new Map<string, string>();
  return {
    getItem: (name: string) => map.get(name) ?? null,
    setItem: (name: string, value: string) => {
      map.set(name, value);
    },
    removeItem: (name: string) => {
      map.delete(name);
    },
  } satisfies SyncStorage;
})();

let appStoreStorage: SyncStorage = inMemoryStorage;

if (typeof globalThis.localStorage !== "undefined") {
  appStoreStorage = globalThis.localStorage;
}

const initialAppState: AppStoreState = {
  allocationFlowStep: Step.PORTFOLIOS,
  currencies: [],
  preferredCurrency: "",
  pfolioFile: "",
};

const normalizeAppState = (
  appState: Partial<AppStoreState> | undefined
): AppStoreState => {
  return {
    allocationFlowStep:
      appState?.allocationFlowStep ?? initialAppState.allocationFlowStep,
    currencies: Array.isArray(appState?.currencies)
      ? appState.currencies
      : initialAppState.currencies,
    preferredCurrency:
      typeof appState?.preferredCurrency === "string"
        ? appState.preferredCurrency
        : initialAppState.preferredCurrency,
    pfolioFile:
      typeof appState?.pfolioFile === "string"
        ? appState.pfolioFile
        : initialAppState.pfolioFile,
  };
};

const parseJSON = (value: unknown): unknown => {
  if (typeof value !== "string") return value;

  try {
    return JSON.parse(value);
  } catch {
    return undefined;
  }
};

const parseLegacyReduxPersistRoot = (
  serialized: string | null
): LegacyReduxPersistRootState | undefined => {
  if (!serialized) return undefined;

  const parsedRoot = parseJSON(serialized);
  if (!parsedRoot || typeof parsedRoot !== "object") return undefined;

  const root = parsedRoot as Record<string, unknown>;
  const app = parseJSON(root.app);
  const pfolio = parseJSON(root.pfolio);
  const persistMeta = parseJSON(root._persist);

  return {
    app: app && typeof app === "object" ? (app as Partial<AppStoreState>) : {},
    pfolio:
      pfolio && typeof pfolio === "object"
        ? (pfolio as LegacyPfolioState)
        : undefined,
    _persist:
      persistMeta && typeof persistMeta === "object"
        ? (persistMeta as LegacyReduxPersistRootState["_persist"])
        : undefined,
  };
};

const extractVersion = (version: unknown): number => {
  return typeof version === "number" ? version : 0;
};

const appStoreMigrations: Record<
  number,
  (state: PersistedAppStoreState) => PersistedAppStoreState
> = {
  0: (state) => state,
  2: (state) => state,
  3: (state) => state,
  4: (state) => {
    const pfolio = state.__legacyPfolio;
    const hasPfolio = typeof pfolio === "object" && pfolio !== null;
    const preferredCurrency =
      typeof pfolio?.quoteCcy === "string" ? pfolio.quoteCcy : "";

    return {
      ...state,
      ...(!hasPfolio && { allocationFlowStep: Step.PORTFOLIOS }),
      preferredCurrency,
    };
  },
  5: (state) => state,
};

const runAppStoreMigrations = (
  state: PersistedAppStoreState,
  persistedVersion: number
): PersistedAppStoreState => {
  const versions = Object.keys(appStoreMigrations)
    .map((v) => Number(v))
    .filter((v) => v > persistedVersion && v <= APP_STORE_PERSIST_VERSION)
    .sort((a, b) => a - b);

  return versions.reduce((next, version) => {
    return appStoreMigrations[version](next);
  }, state);
};

const getLegacyAppStorePersistPayload = (): string | null => {
  const legacyRoot = parseLegacyReduxPersistRoot(
    appStoreStorage.getItem(REDUX_PERSIST_ROOT_KEY)
  );
  if (!legacyRoot) return null;

  const state: PersistedAppStoreState = {
    ...normalizeAppState(legacyRoot.app),
    __legacyPfolio: legacyRoot.pfolio,
  };

  return JSON.stringify({
    state,
    version: extractVersion(legacyRoot._persist?.version),
  });
};

const appStoreStateStorage: StateStorage = {
  getItem: (name) => {
    const appState = appStoreStorage.getItem(name);
    if (appState !== null) {
      return appState;
    }

    if (name !== APP_STORE_PERSIST_KEY) {
      return appState;
    }

    return getLegacyAppStorePersistPayload();
  },
  setItem: (name, value) => appStoreStorage.setItem(name, value),
  removeItem: (name) => appStoreStorage.removeItem(name),
};

export const migrateAppStoreState = (
  persistedState: unknown,
  persistedVersion: number
): AppStoreState => {
  const state =
    persistedState && typeof persistedState === "object"
      ? (persistedState as PersistedAppStoreState)
      : {};

  const migrated = runAppStoreMigrations(state, persistedVersion);
  return normalizeAppState(migrated);
};

export const useAppStore = create<AppStore>()(
  persist(
    (set) => ({
      ...initialAppState,
      setAllocationFlowStep: ({ step }) =>
        set((state) => ({ ...state, allocationFlowStep: step })),
      setCurrencies: ({ currencies }) =>
        set((state) => ({ ...state, currencies: [...currencies] })),
      setPreferredCurrency: ({ ccy }) =>
        set((state) => ({ ...state, preferredCurrency: ccy })),
      setPfolioFile: ({ file }) =>
        set((state) => ({ ...state, pfolioFile: file })),
      hydrateFromRedux: (appState) =>
        set((state) => ({ ...state, ...normalizeAppState(appState) })),
    }),
    {
      name: APP_STORE_PERSIST_KEY,
      version: APP_STORE_PERSIST_VERSION,
      storage: createJSONStorage(() => appStoreStateStorage),
      partialize: (state) => ({
        allocationFlowStep: state.allocationFlowStep,
        currencies: state.currencies,
        preferredCurrency: state.preferredCurrency,
        pfolioFile: state.pfolioFile,
      }),
      migrate: (persistedState, persistedVersion) =>
        migrateAppStoreState(persistedState, persistedVersion),
    }
  )
);

export const bindAppStoreToRedux = (store: ReduxStore): (() => void) => {
  useAppStore.getState().hydrateFromRedux(store.getState().app);

  return store.subscribe(() => {
    useAppStore.getState().hydrateFromRedux(store.getState().app);
  });
};

export const resetAppStoreForTests = () => {
  useAppStore.setState(initialAppState);
  appStoreStorage.removeItem(APP_STORE_PERSIST_KEY);
  appStoreStorage.removeItem(REDUX_PERSIST_ROOT_KEY);
};

export const setAppStoreStorageForTests = (storage: SyncStorage) => {
  appStoreStorage = storage;
};
