import { Step } from "@app/appSlice";
import { create } from "zustand";

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

export const useAppStore = create<AppStore>((set) => ({
  ...initialAppState,
  setAllocationFlowStep: ({ step }) =>
    set((state) => ({ ...state, allocationFlowStep: step })),
  setCurrencies: ({ currencies }) =>
    set((state) => ({ ...state, currencies: [...currencies] })),
  setPreferredCurrency: ({ ccy }) =>
    set((state) => ({ ...state, preferredCurrency: ccy })),
  setPfolioFile: ({ file }) => set((state) => ({ ...state, pfolioFile: file })),
  hydrateFromRedux: (appState) =>
    set((state) => ({ ...state, ...normalizeAppState(appState) })),
}));

export const bindAppStoreToRedux = (store: ReduxStore): (() => void) => {
  useAppStore.getState().hydrateFromRedux(store.getState().app);

  return store.subscribe(() => {
    useAppStore.getState().hydrateFromRedux(store.getState().app);
  });
};

export const resetAppStoreForTests = () => {
  useAppStore.setState(initialAppState);
};
