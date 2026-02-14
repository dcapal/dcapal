import { beforeEach, describe, expect, it } from "vitest";
import { Step } from "@app/appSlice";
import {
  APP_STORE_PERSIST_KEY,
  bindAppStoreToRedux,
  migrateAppStoreState,
  resetAppStoreForTests,
  setAppStoreStorageForTests,
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

type MockStorage = {
  getItem: (name: string) => string | null;
  setItem: (name: string, value: string) => void;
  removeItem: (name: string) => void;
};

const createStorageMock = (): MockStorage => {
  const values = new Map<string, string>();
  return {
    getItem: (name) => values.get(name) ?? null,
    setItem: (name, value) => values.set(name, value),
    removeItem: (name) => values.delete(name),
  };
};

describe("appStore", () => {
  const storage = createStorageMock();

  beforeEach(() => {
    setAppStoreStorageForTests(storage);
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

  it("migrates version 0 payload without app-field changes", () => {
    const migrated = migrateAppStoreState(
      {
        allocationFlowStep: Step.IMPORT,
        currencies: ["usd"],
        preferredCurrency: "usd",
        pfolioFile: "{\"id\":1}",
        __legacyPfolio: { quoteCcy: "usd" },
      },
      0
    );

    expect(migrated).toStrictEqual({
      allocationFlowStep: Step.IMPORT,
      currencies: ["usd"],
      preferredCurrency: "usd",
      pfolioFile: "{\"id\":1}",
    });
  });

  it("migrates version 2 payload without app-field changes", () => {
    const migrated = migrateAppStoreState(
      {
        allocationFlowStep: Step.PORTFOLIO,
        currencies: ["eur"],
        preferredCurrency: "eur",
        pfolioFile: "{\"id\":2}",
        __legacyPfolio: { quoteCcy: "eur" },
      },
      2
    );

    expect(migrated).toStrictEqual({
      allocationFlowStep: Step.PORTFOLIO,
      currencies: ["eur"],
      preferredCurrency: "eur",
      pfolioFile: "{\"id\":2}",
    });
  });

  it("migrates version 3 payload without app-field changes", () => {
    const migrated = migrateAppStoreState(
      {
        allocationFlowStep: Step.END,
        currencies: ["chf"],
        preferredCurrency: "chf",
        pfolioFile: "{\"id\":3}",
        __legacyPfolio: { quoteCcy: "chf" },
      },
      3
    );

    expect(migrated).toStrictEqual({
      allocationFlowStep: Step.END,
      currencies: ["chf"],
      preferredCurrency: "chf",
      pfolioFile: "{\"id\":3}",
    });
  });

  it("forces portfolios step and empty preferred currency when legacy pfolio is missing", () => {
    const migrated = migrateAppStoreState(
      {
        allocationFlowStep: Step.IMPORT,
        currencies: ["usd"],
        preferredCurrency: "eur",
        pfolioFile: "{\"id\":4}",
      },
      3
    );

    expect(migrated.allocationFlowStep).toBe(Step.PORTFOLIOS);
    expect(migrated.preferredCurrency).toBe("");
    expect(migrated.currencies).toStrictEqual(["usd"]);
    expect(migrated.pfolioFile).toBe("{\"id\":4}");
  });

  it("keeps step and uses quoteCcy when legacy pfolio object exists even with empty assets", () => {
    const migrated = migrateAppStoreState(
      {
        allocationFlowStep: Step.IMPORT,
        currencies: ["usd"],
        preferredCurrency: "eur",
        pfolioFile: "{\"id\":5}",
        __legacyPfolio: { assets: {}, quoteCcy: "usd" },
      },
      3
    );

    expect(migrated.allocationFlowStep).toBe(Step.IMPORT);
    expect(migrated.preferredCurrency).toBe("usd");
    expect(migrated.currencies).toStrictEqual(["usd"]);
    expect(migrated.pfolioFile).toBe("{\"id\":5}");
  });

  it("sets empty preferred currency when legacy pfolio has no quoteCcy", () => {
    const migrated = migrateAppStoreState(
      {
        allocationFlowStep: Step.PORTFOLIO,
        currencies: ["usd", "eur"],
        preferredCurrency: "usd",
        pfolioFile: "{\"id\":6}",
        __legacyPfolio: { assets: { VWCE: { symbol: "VWCE" } } },
      },
      3
    );

    expect(migrated.allocationFlowStep).toBe(Step.PORTFOLIO);
    expect(migrated.preferredCurrency).toBe("");
  });

  it("keeps version 5 payload unchanged", () => {
    const migrated = migrateAppStoreState(
      {
        allocationFlowStep: Step.PORTFOLIOS,
        currencies: ["gbp"],
        preferredCurrency: "gbp",
        pfolioFile: "{\"id\":6}",
      },
      5
    );

    expect(migrated).toStrictEqual({
      allocationFlowStep: Step.PORTFOLIOS,
      currencies: ["gbp"],
      preferredCurrency: "gbp",
      pfolioFile: "{\"id\":6}",
    });
  });

  it("normalizes malformed persisted fields", () => {
    const migrated = migrateAppStoreState(
      {
        allocationFlowStep: undefined,
        currencies: "usd",
        preferredCurrency: null,
        pfolioFile: 1234,
      },
      2
    );

    expect(migrated).toStrictEqual({
      allocationFlowStep: Step.PORTFOLIOS,
      currencies: [],
      preferredCurrency: "",
      pfolioFile: "",
    });
  });

  it("rehydrates from legacy redux persist payload with pfolio object", async () => {
    const persistedRoot = {
      app: JSON.stringify({
        allocationFlowStep: Step.IMPORT,
        currencies: ["usd", "eur"],
        preferredCurrency: "",
        pfolioFile: "{\"name\":\"imported\"}",
      }),
      pfolio: JSON.stringify({
        assets: {},
        quoteCcy: "usd",
      }),
      _persist: JSON.stringify({ version: 3, rehydrated: true }),
    };
    storage.setItem("persist:root", JSON.stringify(persistedRoot));

    useAppStore.setState({
      ...useAppStore.getState(),
      allocationFlowStep: Step.END,
      currencies: ["cad"],
      preferredCurrency: "cad",
      pfolioFile: "x",
    });
    storage.removeItem(APP_STORE_PERSIST_KEY);

    await useAppStore.persist.rehydrate();

    expect(useAppStore.getState().allocationFlowStep).toBe(Step.IMPORT);
    expect(useAppStore.getState().currencies).toStrictEqual(["usd", "eur"]);
    expect(useAppStore.getState().preferredCurrency).toBe("usd");
    expect(useAppStore.getState().pfolioFile).toBe("{\"name\":\"imported\"}");
  });

  it("rehydrates from legacy redux persist payload without pfolio object", async () => {
    const persistedRoot = {
      app: JSON.stringify({
        allocationFlowStep: Step.IMPORT,
        currencies: ["usd", "eur"],
        preferredCurrency: "chf",
        pfolioFile: "{\"name\":\"imported\"}",
      }),
      _persist: JSON.stringify({ version: 3, rehydrated: true }),
    };
    storage.setItem("persist:root", JSON.stringify(persistedRoot));

    useAppStore.setState({
      ...useAppStore.getState(),
      allocationFlowStep: Step.END,
      currencies: ["cad"],
      preferredCurrency: "cad",
      pfolioFile: "x",
    });
    storage.removeItem(APP_STORE_PERSIST_KEY);

    await useAppStore.persist.rehydrate();

    expect(useAppStore.getState().allocationFlowStep).toBe(Step.PORTFOLIOS);
    expect(useAppStore.getState().currencies).toStrictEqual(["usd", "eur"]);
    expect(useAppStore.getState().preferredCurrency).toBe("");
    expect(useAppStore.getState().pfolioFile).toBe("{\"name\":\"imported\"}");
  });

  it("prefers redux bridge updates over persisted zustand values", async () => {
    storage.setItem(
      APP_STORE_PERSIST_KEY,
      JSON.stringify({
        state: {
          allocationFlowStep: Step.END,
          currencies: ["cad"],
          preferredCurrency: "cad",
          pfolioFile: "{\"persisted\":true}",
        },
        version: 5,
      })
    );

    await useAppStore.persist.rehydrate();
    expect(useAppStore.getState().preferredCurrency).toBe("cad");

    const reduxStore = createReduxStoreMock({
      allocationFlowStep: Step.IMPORT,
      currencies: ["usd"],
      preferredCurrency: "usd",
      pfolioFile: "{\"redux\":true}",
    });
    bindAppStoreToRedux(reduxStore);

    expect(useAppStore.getState().allocationFlowStep).toBe(Step.IMPORT);
    expect(useAppStore.getState().currencies).toStrictEqual(["usd"]);
    expect(useAppStore.getState().preferredCurrency).toBe("usd");
    expect(useAppStore.getState().pfolioFile).toBe("{\"redux\":true}");
  });
});
