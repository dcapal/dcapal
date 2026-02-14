import { beforeEach, describe, expect, it } from "vitest";
import { Step } from "@app/appSlice";
import {
  bindAppStoreToRedux,
  resetAppStoreForTests,
  useAppStore,
  type AppStoreState,
} from "./appStore";

type ReduxState = { app: AppStoreState };

type ReduxStoreMock = {
  getState: () => ReduxState;
  subscribe: (listener: () => void) => () => void;
  updateApp: (appState: AppStoreState) => void;
};

const createReduxStoreMock = (initialAppState: AppStoreState): ReduxStoreMock => {
  let state: ReduxState = { app: initialAppState };
  const listeners = new Set<() => void>();

  return {
    getState: () => state,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    updateApp: (appState) => {
      state = { app: appState };
      listeners.forEach((listener) => listener());
    },
  };
};

describe("appStore", () => {
  beforeEach(() => {
    resetAppStoreForTests();
  });

  it("initializes with expected defaults", () => {
    const state = useAppStore.getState();

    expect(state.allocationFlowStep).toBe(Step.PORTFOLIOS);
    expect(state.currencies).toStrictEqual([]);
    expect(state.preferredCurrency).toBe("");
    expect(state.pfolioFile).toBe("");
  });

  it("updates state through local actions", () => {
    const store = useAppStore.getState();

    store.setAllocationFlowStep({ step: Step.IMPORT });
    store.setCurrencies({ currencies: ["usd", "eur"] });
    store.setPreferredCurrency({ ccy: "usd" });
    store.setPfolioFile({ file: "{\"name\":\"demo\"}" });

    const next = useAppStore.getState();
    expect(next.allocationFlowStep).toBe(Step.IMPORT);
    expect(next.currencies).toStrictEqual(["usd", "eur"]);
    expect(next.preferredCurrency).toBe("usd");
    expect(next.pfolioFile).toBe("{\"name\":\"demo\"}");
  });

  it("hydrates from redux app state", () => {
    useAppStore.getState().hydrateFromRedux({
      allocationFlowStep: Step.END,
      currencies: ["chf"],
      preferredCurrency: "chf",
      pfolioFile: "{\"name\":\"imported\"}",
    });

    const next = useAppStore.getState();
    expect(next.allocationFlowStep).toBe(Step.END);
    expect(next.currencies).toStrictEqual(["chf"]);
    expect(next.preferredCurrency).toBe("chf");
    expect(next.pfolioFile).toBe("{\"name\":\"imported\"}");
  });

  it("mirrors redux changes through bridge and stops after unsubscribe", () => {
    const reduxStore = createReduxStoreMock({
      allocationFlowStep: Step.PORTFOLIOS,
      currencies: ["eur"],
      preferredCurrency: "eur",
      pfolioFile: "",
    });

    const unsubscribe = bindAppStoreToRedux(reduxStore);

    expect(useAppStore.getState().currencies).toStrictEqual(["eur"]);
    expect(useAppStore.getState().preferredCurrency).toBe("eur");

    reduxStore.updateApp({
      allocationFlowStep: Step.IMPORT,
      currencies: ["usd"],
      preferredCurrency: "usd",
      pfolioFile: "{\"id\":1}",
    });

    expect(useAppStore.getState().allocationFlowStep).toBe(Step.IMPORT);
    expect(useAppStore.getState().currencies).toStrictEqual(["usd"]);
    expect(useAppStore.getState().preferredCurrency).toBe("usd");
    expect(useAppStore.getState().pfolioFile).toBe("{\"id\":1}");

    unsubscribe();

    reduxStore.updateApp({
      allocationFlowStep: Step.END,
      currencies: ["cad"],
      preferredCurrency: "cad",
      pfolioFile: "{\"id\":2}",
    });

    expect(useAppStore.getState().allocationFlowStep).toBe(Step.IMPORT);
    expect(useAppStore.getState().currencies).toStrictEqual(["usd"]);
    expect(useAppStore.getState().preferredCurrency).toBe("usd");
    expect(useAppStore.getState().pfolioFile).toBe("{\"id\":1}");
  });
});
