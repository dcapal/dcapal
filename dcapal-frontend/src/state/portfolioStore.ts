import i18n from "i18next";
import { create } from "zustand";
import {
  createJSONStorage,
  persist,
  type StateStorage,
} from "zustand/middleware";
import { syncPortfoliosAPI } from "@/api";
import { REFRESH_PRICE_INTERVAL_SEC } from "@app/config";
import { roundAmount, roundPrice } from "@utils/index.js";
import {
  ACLASS,
  FeeType,
  aclassToString,
  currentPortfolio,
  feeTypeToString,
  getDefaultFees,
  getNewPortfolio,
  isWholeShares,
  parseAClass,
  parseFeeType,
  parseFees,
  type Portfolio,
  type PortfolioAsset,
  type PortfolioFees,
  type PortfolioStoreState,
} from "./portfolioDomain";

type SyncStorage = {
  getItem: (name: string) => string | null;
  setItem: (name: string, value: string) => void;
  removeItem: (name: string) => void;
};

type LegacyReduxPersistRootState = {
  pfolio?: unknown;
  _persist?: {
    version?: unknown;
  };
};

type LegacySinglePortfolioState = Partial<Portfolio> & {
  assets?: Record<string, PortfolioAsset>;
  quoteCcy?: string;
};

type SyncPortfoliosPayload = {
  updatedPortfolios?: Array<Record<string, any>>;
  deletedPortfolios?: string[];
} | null;

export type PortfolioStoreActions = {
  addPortfolio: (payload: { pfolio: Portfolio }) => void;
  deletePortfolio: (payload: { id: string }) => void;
  duplicatePortfolio: (payload: { id: string }) => void;
  selectPortfolio: (payload: { id: string | null }) => void;
  renamePortfolio: (payload: { id: string; name: string }) => void;
  addAsset: (payload: {
    symbol: string;
    name: string;
    aclass: number;
    baseCcy: string;
    price: number;
    provider: string;
  }) => void;
  removeAsset: (payload: { symbol: string }) => void;
  setQty: (payload: { symbol: string; qty: number }) => void;
  setPrice: (payload: { symbol: string; price: number }) => void;
  setTargetWeight: (payload: { symbol: string; weight: number }) => void;
  setRefreshTime: (payload: { time: number }) => void;
  setQuoteCurrency: (payload: { quoteCcy: string }) => void;
  setBudget: (payload: { budget: number }) => void;
  setFees: (payload: { fees: PortfolioFees }) => void;
  setFeesAsset: (payload: { symbol: string; fees: PortfolioFees }) => void;
  setFeeType: (payload: { type: number | null }) => void;
  setFeeTypeAsset: (payload: { symbol: string; type: number | null }) => void;
  setMaxFeeImpact: (payload: { value: number | null }) => void;
  setMaxFeeImpactAsset: (payload: { symbol: string; value: number | null }) => void;
  setFixedFeeAmount: (payload: { value: number | null }) => void;
  setFixedFeeAmountAsset: (payload: { symbol: string; value: number | null }) => void;
  setVariableFee: (payload: {
    feeRate?: number | null;
    minFee?: number | null;
    maxFee?: number | null;
  }) => void;
  setVariableFeeAsset: (payload: {
    symbol: string;
    feeRate?: number | null;
    minFee?: number | null;
    maxFee?: number | null;
  }) => void;
  clearBudget: () => void;
  syncPortfoliosNow: () => Promise<void>;
};

export type PortfolioStore = PortfolioStoreState & PortfolioStoreActions;

export const PORTFOLIO_STORE_PERSIST_KEY = "zustand:portfolioStore";
export const PORTFOLIO_STORE_PERSIST_VERSION = 5;

const REDUX_PERSIST_ROOT_KEY = "persist:root";

const mapValues = <TInput, TOutput>(
  source: Record<string, TInput>,
  mapper: (value: TInput) => TOutput
): Record<string, TOutput> => {
  return Object.entries(source).reduce(
    (acc, [key, value]) => {
      acc[key] = mapper(value);
      return acc;
    },
    {} as Record<string, TOutput>
  );
};

const updateWeight = (asset: PortfolioAsset, totalAmount: number) => {
  const qty = asset.qty || 0;
  const price = asset.price || 0;
  const amount = qty * price;

  const weight = totalAmount > 0 ? amount / totalAmount : 0;
  asset.weight = weight * 100;
};

const initialPortfolioState = (): PortfolioStoreState => {
  return {
    selected: null,
    pfolios: {},
    deletedPortfolios: [],
  };
};

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

let portfolioStoreStorage: SyncStorage = inMemoryStorage;

if (typeof globalThis.localStorage !== "undefined") {
  portfolioStoreStorage = globalThis.localStorage;
}

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
  return {
    pfolio: parseJSON(root.pfolio),
    _persist: parseJSON(root._persist) as LegacyReduxPersistRootState["_persist"],
  };
};

const extractVersion = (version: unknown): number => {
  return typeof version === "number" ? version : 0;
};

const isPortfolioStoreState = (
  value: unknown
): value is PortfolioStoreState => {
  return Boolean(
    value &&
      typeof value === "object" &&
      "selected" in value &&
      "pfolios" in value
  );
};

const normalizeAsset = (asset: Partial<PortfolioAsset>, fallbackSymbol: string): PortfolioAsset => {
  return {
    idx: typeof asset.idx === "number" ? asset.idx : 0,
    symbol: typeof asset.symbol === "string" ? asset.symbol : fallbackSymbol,
    name: typeof asset.name === "string" ? asset.name : fallbackSymbol,
    aclass: typeof asset.aclass === "number" ? asset.aclass : ACLASS.UNDEFINED,
    baseCcy: typeof asset.baseCcy === "string" ? asset.baseCcy : "",
    provider: typeof asset.provider === "string" ? asset.provider : "",
    qty: typeof asset.qty === "number" ? asset.qty : 0,
    targetWeight: typeof asset.targetWeight === "number" ? asset.targetWeight : 0,
    price: typeof asset.price === "number" ? asset.price : 0,
    amount: typeof asset.amount === "number" ? asset.amount : 0,
    weight: typeof asset.weight === "number" ? asset.weight : 0,
    fees:
      asset.fees === null || typeof asset.fees === "object" ? asset.fees : null,
  };
};

const normalizePortfolio = (input: Partial<Portfolio>, id: string): Portfolio => {
  const assetsInput =
    input.assets && typeof input.assets === "object" ? input.assets : {};
  const assets = Object.entries(assetsInput).reduce(
    (acc, [symbol, raw]) => {
      acc[symbol] = normalizeAsset(raw, symbol);
      return acc;
    },
    {} as Record<string, PortfolioAsset>
  );

  return {
    id,
    name: typeof input.name === "string" ? input.name : "",
    assets,
    quoteCcy: typeof input.quoteCcy === "string" ? input.quoteCcy : "eur",
    nextIdx: typeof input.nextIdx === "number" ? input.nextIdx : 0,
    totalAmount: typeof input.totalAmount === "number" ? input.totalAmount : 0,
    budget: typeof input.budget === "number" ? input.budget : 0,
    fees:
      input.fees === null ||
      (typeof input.fees === "object" && input.fees !== undefined)
        ? input.fees
        : getDefaultFees(FeeType.ZERO_FEE),
    lastPriceRefresh:
      typeof input.lastPriceRefresh === "number"
        ? input.lastPriceRefresh
        : Date.now(),
    lastUpdatedAt:
      typeof input.lastUpdatedAt === "number" ? input.lastUpdatedAt : Date.now(),
  };
};

const normalizePortfolioStoreState = (state: unknown): PortfolioStoreState => {
  if (!isPortfolioStoreState(state)) {
    return initialPortfolioState();
  }

  const input = state as PortfolioStoreState;
  const pfolios = Object.entries(input.pfolios || {}).reduce(
    (acc, [id, pfolio]) => {
      acc[id] = normalizePortfolio(pfolio, id);
      return acc;
    },
    {} as Record<string, Portfolio>
  );

  const selected =
    typeof input.selected === "string" && input.selected in pfolios
      ? input.selected
      : null;

  const deletedPortfolios = Array.isArray(input.deletedPortfolios)
    ? input.deletedPortfolios.filter((id): id is string => typeof id === "string")
    : [];

  return {
    selected,
    pfolios,
    deletedPortfolios,
  };
};

const portfolioStoreMigrations: Record<number, (state: unknown) => unknown> = {
  0: (state) => {
    const pfolio = state as LegacySinglePortfolioState;
    const assets = pfolio?.assets;
    if (!assets || Object.keys(assets).length === 0) {
      return state;
    }

    const anyAsset = Object.values(assets)[0] as Record<string, unknown> | undefined;
    if (anyAsset && !("aclass" in anyAsset)) {
      return {};
    }

    return state;
  },
  2: (state) => {
    if (isPortfolioStoreState(state)) {
      return state;
    }

    const pfolio = state as LegacySinglePortfolioState;
    return {
      ...pfolio,
      fees: getDefaultFees(FeeType.ZERO_FEE),
      assets: mapValues(pfolio.assets || {}, (asset) => ({ ...asset, fees: null })),
    };
  },
  3: (state) => {
    if (isPortfolioStoreState(state)) {
      return state;
    }

    const pfolio = state as LegacySinglePortfolioState;
    return {
      ...pfolio,
      lastPriceRefresh: Date.now() - (REFRESH_PRICE_INTERVAL_SEC + 1) * 1000,
    };
  },
  4: (state) => {
    if (isPortfolioStoreState(state)) {
      return state;
    }

    const pfolio = state as LegacySinglePortfolioState;
    if (!pfolio || typeof pfolio !== "object" || Object.keys(pfolio).length === 0) {
      return initialPortfolioState();
    }

    const id = typeof pfolio.id === "string" && pfolio.id ? pfolio.id : crypto.randomUUID();
    const name =
      typeof pfolio.name === "string" && pfolio.name
        ? pfolio.name
        : i18n.t("importStep.defaultPortfolioName");

    return {
      selected: id,
      pfolios: {
        [id]: {
          ...pfolio,
          id,
          name,
        },
      },
      deletedPortfolios: [],
    };
  },
  5: (state) => {
    if (!isPortfolioStoreState(state)) {
      return state;
    }

    const pfolioState = state as PortfolioStoreState;
    return {
      ...pfolioState,
      pfolios: Object.keys(pfolioState.pfolios || {}).reduce(
        (acc, key) => {
          acc[key] = {
            ...pfolioState.pfolios[key],
            lastUpdatedAt:
              pfolioState.pfolios[key]?.lastUpdatedAt || Date.now(),
          };
          return acc;
        },
        {} as Record<string, Portfolio>
      ),
      deletedPortfolios: Array.isArray(pfolioState.deletedPortfolios)
        ? pfolioState.deletedPortfolios
        : [],
    };
  },
};

const runPortfolioStoreMigrations = (
  state: unknown,
  persistedVersion: number
): unknown => {
  const versions = Object.keys(portfolioStoreMigrations)
    .map(Number)
    .filter(
      (version) =>
        version > persistedVersion && version <= PORTFOLIO_STORE_PERSIST_VERSION
    )
    .sort((a, b) => a - b);

  return versions.reduce((nextState, version) => {
    return portfolioStoreMigrations[version](nextState);
  }, state);
};

const getLegacyPortfolioStorePersistPayload = (): string | null => {
  const legacyRoot = parseLegacyReduxPersistRoot(
    portfolioStoreStorage.getItem(REDUX_PERSIST_ROOT_KEY)
  );

  if (!legacyRoot || legacyRoot.pfolio == null) {
    return null;
  }

  return JSON.stringify({
    state: legacyRoot.pfolio,
    version: extractVersion(legacyRoot._persist?.version),
  });
};

const portfolioStateStorage: StateStorage = {
  getItem: (name) => {
    const state = portfolioStoreStorage.getItem(name);
    if (state !== null) {
      return state;
    }

    if (name !== PORTFOLIO_STORE_PERSIST_KEY) {
      return state;
    }

    return getLegacyPortfolioStorePersistPayload();
  },
  setItem: (name, value) => portfolioStoreStorage.setItem(name, value),
  removeItem: (name) => portfolioStoreStorage.removeItem(name),
};

export const migratePortfolioStoreState = (
  persistedState: unknown,
  persistedVersion: number
): PortfolioStoreState => {
  const migratedState = runPortfolioStoreMigrations(
    persistedState ?? {},
    persistedVersion
  );
  return normalizePortfolioStoreState(migratedState);
};

export const applySyncPortfoliosResult = (
  state: PortfolioStoreState,
  payload: SyncPortfoliosPayload
): PortfolioStoreState => {
  if (!payload) return state;

  const nextPfolios = { ...state.pfolios };
  payload.updatedPortfolios?.forEach((pf) => {
    const id = String(pf.id);
    const previous = nextPfolios[id];
    nextPfolios[id] = {
      id,
      name: typeof pf.name === "string" ? pf.name : previous?.name || "",
      fees: parseFees(pf.fees),
      quoteCcy:
        typeof pf.quoteCcy === "string" ? pf.quoteCcy : previous?.quoteCcy || "",
      assets: Array.isArray(pf.assets)
        ? pf.assets.reduce((acc: Record<string, PortfolioAsset>, asset: any) => {
            const symbol = String(asset.symbol);
            acc[symbol] = {
              ...asset,
              symbol,
              fees: parseFees(asset.fees),
              aclass: parseAClass(asset.aclass),
              weight: asset.targetWeight,
              amount: 0,
            };
            return acc;
          }, {})
        : {},
      nextIdx: pf.nextIdx || 0,
      totalAmount: pf.totalAmount || 0,
      budget: pf.budget || 0,
      lastPriceRefresh: previous?.lastPriceRefresh || Date.now(),
      lastUpdatedAt:
        typeof pf.lastUpdatedAt === "number"
          ? pf.lastUpdatedAt
          : typeof pf.lastUpdatedAt === "string"
            ? new Date(pf.lastUpdatedAt).getTime()
            : previous?.lastUpdatedAt || Date.now(),
    };
  });

  payload.deletedPortfolios?.forEach((id) => {
    if (id in nextPfolios) {
      delete nextPfolios[id];
    }
  });

  return {
    ...state,
    pfolios: nextPfolios,
  };
};

export const usePortfolioStore = create<PortfolioStore>()(
  persist(
    (set, get) => ({
      ...initialPortfolioState(),
      addPortfolio: ({ pfolio }) =>
        set((state) => ({
          ...state,
          pfolios: { ...state.pfolios, [pfolio.id]: pfolio },
        })),
      deletePortfolio: ({ id }) =>
        set((state) => {
          if (!(id in state.pfolios)) return state;

          const nextPfolios = { ...state.pfolios };
          delete nextPfolios[id];

          return {
            ...state,
            pfolios: nextPfolios,
            deletedPortfolios: [...state.deletedPortfolios, id],
          };
        }),
      duplicatePortfolio: ({ id }) =>
        set((state) => {
          if (!(id in state.pfolios)) return state;

          const pfolio = { ...state.pfolios[id] };
          pfolio.id = crypto.randomUUID();
          pfolio.name += " " + i18n.t("portfoliosStep.copy");

          return {
            ...state,
            pfolios: { ...state.pfolios, [pfolio.id]: pfolio },
          };
        }),
      selectPortfolio: ({ id }) =>
        set((state) => {
          if (!id) {
            return { ...state, selected: null };
          }

          if (id === state.selected) return state;
          if (!(id in state.pfolios)) return state;

          return { ...state, selected: id };
        }),
      renamePortfolio: ({ id, name }) =>
        set((state) => {
          if (!(id in state.pfolios)) return state;

          return {
            ...state,
            pfolios: {
              ...state.pfolios,
              [id]: {
                ...state.pfolios[id],
                name,
                lastUpdatedAt: Date.now(),
              },
            },
          };
        }),
      addAsset: (payload) =>
        set((state) => {
          const pfolio = currentPortfolio(state);
          if (!pfolio) return state;

          const symbol = payload.symbol;
          if (symbol && symbol in pfolio.assets) return state;

          const legacyIdx = (state as unknown as { nextIdx?: number }).nextIdx;
          const nextPfolio: Portfolio = {
            ...pfolio,
            assets: {
              ...pfolio.assets,
              [payload.symbol]: {
                idx: legacyIdx as number,
                symbol: payload.symbol,
                name: payload.name,
                aclass: payload.aclass,
                baseCcy: payload.baseCcy,
                price: roundPrice(payload.price) || 0,
                provider: payload.provider,
                qty: 0,
                amount: 0,
                weight: 0,
                targetWeight: 0,
                fees: null,
              },
            },
            nextIdx: pfolio.nextIdx + 1,
            lastUpdatedAt: Date.now(),
          };

          if (Object.keys(nextPfolio.assets).length === 1) {
            nextPfolio.lastPriceRefresh = Date.now();
          }

          return {
            ...state,
            pfolios: { ...state.pfolios, [pfolio.id]: nextPfolio },
          };
        }),
      removeAsset: ({ symbol }) =>
        set((state) => {
          const pfolio = currentPortfolio(state);
          if (!pfolio) return state;

          const nextPfolio: Portfolio = {
            ...pfolio,
            assets: { ...pfolio.assets },
            totalAmount: pfolio.totalAmount,
            lastUpdatedAt: Date.now(),
          };

          if (symbol in nextPfolio.assets) {
            const asset = nextPfolio.assets[symbol];
            nextPfolio.totalAmount -= asset.amount;
            delete nextPfolio.assets[symbol];

            Object.values(nextPfolio.assets).forEach((a) => {
              updateWeight(a, nextPfolio.totalAmount);
            });
          }

          return {
            ...state,
            pfolios: { ...state.pfolios, [pfolio.id]: nextPfolio },
          };
        }),
      setQty: ({ symbol, qty }) =>
        set((state) => {
          const pfolio = currentPortfolio(state);
          if (!pfolio) return state;

          const asset = pfolio.assets[symbol]!;
          const price = asset?.price || 0;
          const newAmount = roundAmount((qty || 0) * price);
          const nextTotalAmount = pfolio.totalAmount - asset.amount + newAmount;

          const nextPfolio: Portfolio = {
            ...pfolio,
            totalAmount: nextTotalAmount,
            assets: {
              ...pfolio.assets,
              [symbol]: {
                ...pfolio.assets[symbol],
                qty: qty || 0,
                amount: newAmount,
              },
            },
            lastUpdatedAt: Date.now(),
          };

          Object.values(nextPfolio.assets).forEach((a) => {
            updateWeight(a, nextPfolio.totalAmount);
          });

          return {
            ...state,
            pfolios: { ...state.pfolios, [pfolio.id]: nextPfolio },
          };
        }),
      setPrice: ({ symbol, price }) =>
        set((state) => {
          const pfolio = currentPortfolio(state);
          if (!pfolio) return state;

          if (!symbol || !(symbol in pfolio.assets)) {
            return state;
          }

          const parsedPrice = roundPrice(price);
          const asset = pfolio.assets[symbol];
          const newAmount = roundAmount((asset.qty || 0) * parsedPrice);
          const nextTotalAmount = pfolio.totalAmount - asset.amount + newAmount;

          const nextPfolio: Portfolio = {
            ...pfolio,
            totalAmount: nextTotalAmount,
            assets: {
              ...pfolio.assets,
              [symbol]: {
                ...asset,
                price: parsedPrice,
                amount: newAmount,
              },
            },
            lastUpdatedAt: Date.now(),
          };

          Object.values(nextPfolio.assets).forEach((a) => {
            updateWeight(a, nextPfolio.totalAmount);
          });

          return {
            ...state,
            pfolios: { ...state.pfolios, [pfolio.id]: nextPfolio },
          };
        }),
      setTargetWeight: ({ symbol, weight }) =>
        set((state) => {
          const pfolio = currentPortfolio(state);
          if (!pfolio) return state;

          const nextPfolio: Portfolio = {
            ...pfolio,
            assets: {
              ...pfolio.assets,
              [symbol]: {
                ...pfolio.assets[symbol],
                targetWeight: weight || 0,
              },
            },
            lastUpdatedAt: Date.now(),
          };

          return {
            ...state,
            pfolios: { ...state.pfolios, [pfolio.id]: nextPfolio },
          };
        }),
      setRefreshTime: ({ time }) =>
        set((state) => {
          const pfolio = currentPortfolio(state);
          if (!pfolio) return state;

          const nextPfolio: Portfolio = {
            ...pfolio,
            lastUpdatedAt: Date.now(),
          };

          if (time) {
            nextPfolio.lastPriceRefresh = time;
          }

          return {
            ...state,
            pfolios: { ...state.pfolios, [pfolio.id]: nextPfolio },
          };
        }),
      setQuoteCurrency: ({ quoteCcy }) =>
        set((state) => {
          const pfolio = currentPortfolio(state);
          if (!pfolio) return state;

          const nextPfolio: Portfolio = {
            ...pfolio,
            lastUpdatedAt: Date.now(),
          };

          if (quoteCcy && quoteCcy.length > 0) {
            nextPfolio.quoteCcy = quoteCcy;
          }

          return {
            ...state,
            pfolios: { ...state.pfolios, [pfolio.id]: nextPfolio },
          };
        }),
      setBudget: ({ budget }) =>
        set((state) => {
          const pfolio = currentPortfolio(state);
          if (!pfolio) return state;

          const nextPfolio: Portfolio = {
            ...pfolio,
            lastUpdatedAt: Date.now(),
          };

          if (budget && budget >= 0) {
            nextPfolio.budget = budget;
          }

          return {
            ...state,
            pfolios: { ...state.pfolios, [pfolio.id]: nextPfolio },
          };
        }),
      setFees: ({ fees }) =>
        set((state) => {
          const pfolio = currentPortfolio(state);
          if (!pfolio) return state;
          if (!fees) return state;

          const nextPfolio: Portfolio = {
            ...pfolio,
            fees,
            lastUpdatedAt: Date.now(),
          };

          return {
            ...state,
            pfolios: { ...state.pfolios, [pfolio.id]: nextPfolio },
          };
        }),
      setFeesAsset: ({ symbol, fees }) =>
        set((state) => {
          const pfolio = currentPortfolio(state);
          if (!pfolio) return state;
          if (!symbol || !(symbol in pfolio.assets)) return state;

          const nextPfolio: Portfolio = {
            ...pfolio,
            assets: {
              ...pfolio.assets,
              [symbol]: {
                ...pfolio.assets[symbol],
                fees,
              },
            },
            lastUpdatedAt: Date.now(),
          };

          return {
            ...state,
            pfolios: { ...state.pfolios, [pfolio.id]: nextPfolio },
          };
        }),
      setFeeType: ({ type }) =>
        set((state) => {
          const pfolio = currentPortfolio(state);
          if (!pfolio) return state;

          if (!type) {
            return {
              ...state,
              pfolios: {
                ...state.pfolios,
                [pfolio.id]: {
                  ...pfolio,
                  fees: getDefaultFees(FeeType.ZERO_FEE),
                },
              },
            };
          }

          if (!pfolio.fees?.feeStructure?.type) {
            return {
              ...state,
              pfolios: {
                ...state.pfolios,
                [pfolio.id]: {
                  ...pfolio,
                  fees: getDefaultFees(type),
                },
              },
            };
          }

          let nextFees = pfolio.fees;
          if (type !== pfolio.fees.feeStructure.type) {
            nextFees = {
              ...pfolio.fees,
              feeStructure: getDefaultFees(type)?.feeStructure || {
                type: FeeType.ZERO_FEE,
              },
            };
          }

          return {
            ...state,
            pfolios: {
              ...state.pfolios,
              [pfolio.id]: {
                ...pfolio,
                fees: nextFees,
                lastUpdatedAt: Date.now(),
              },
            },
          };
        }),
      setFeeTypeAsset: ({ symbol, type }) =>
        set((state) => {
          const pfolio = currentPortfolio(state);
          if (!pfolio) return state;
          if (!symbol || !(symbol in pfolio.assets)) return state;

          if (!type) {
            return {
              ...state,
              pfolios: {
                ...state.pfolios,
                [pfolio.id]: {
                  ...pfolio,
                  assets: {
                    ...pfolio.assets,
                    [symbol]: {
                      ...pfolio.assets[symbol],
                      fees: null,
                    },
                  },
                },
              },
            };
          }

          const asset = pfolio.assets[symbol];
          if (!asset.fees?.feeStructure?.type) {
            return {
              ...state,
              pfolios: {
                ...state.pfolios,
                [pfolio.id]: {
                  ...pfolio,
                  assets: {
                    ...pfolio.assets,
                    [symbol]: {
                      ...asset,
                      fees: getDefaultFees(type),
                    },
                  },
                },
              },
            };
          }

          let nextFees = asset.fees;
          if (type !== asset.fees.feeStructure.type) {
            nextFees = {
              ...asset.fees,
              feeStructure: getDefaultFees(type)?.feeStructure || {
                type: FeeType.ZERO_FEE,
              },
            };
          }

          return {
            ...state,
            pfolios: {
              ...state.pfolios,
              [pfolio.id]: {
                ...pfolio,
                assets: {
                  ...pfolio.assets,
                  [symbol]: {
                    ...asset,
                    fees: nextFees,
                  },
                },
                lastUpdatedAt: Date.now(),
              },
            },
          };
        }),
      setMaxFeeImpact: ({ value }) =>
        set((state) => {
          const pfolio = currentPortfolio(state);
          if (!pfolio) return state;
          if (!pfolio.fees) return state;

          return {
            ...state,
            pfolios: {
              ...state.pfolios,
              [pfolio.id]: {
                ...pfolio,
                fees: {
                  ...pfolio.fees,
                  maxFeeImpact: value,
                },
                lastUpdatedAt: Date.now(),
              },
            },
          };
        }),
      setMaxFeeImpactAsset: ({ symbol, value }) =>
        set((state) => {
          const pfolio = currentPortfolio(state);
          if (!pfolio) return state;
          if (!symbol || !(symbol in pfolio.assets)) return state;

          const fees = pfolio.assets[symbol].fees;
          if (!fees) return state;

          return {
            ...state,
            pfolios: {
              ...state.pfolios,
              [pfolio.id]: {
                ...pfolio,
                assets: {
                  ...pfolio.assets,
                  [symbol]: {
                    ...pfolio.assets[symbol],
                    fees: {
                      ...fees,
                      maxFeeImpact: value,
                    },
                  },
                },
                lastUpdatedAt: Date.now(),
              },
            },
          };
        }),
      setFixedFeeAmount: ({ value }) =>
        set((state) => {
          const pfolio = currentPortfolio(state);
          if (!pfolio) return state;
          if (!(pfolio.fees && pfolio.fees.feeStructure?.type === FeeType.FIXED)) {
            return state;
          }

          return {
            ...state,
            pfolios: {
              ...state.pfolios,
              [pfolio.id]: {
                ...pfolio,
                fees: {
                  ...pfolio.fees,
                  feeStructure: {
                    ...pfolio.fees.feeStructure,
                    feeAmount: value || 0,
                  },
                },
                lastUpdatedAt: Date.now(),
              },
            },
          };
        }),
      setFixedFeeAmountAsset: ({ symbol, value }) =>
        set((state) => {
          const pfolio = currentPortfolio(state);
          if (!pfolio) return state;
          if (!symbol || !(symbol in pfolio.assets)) return state;

          const fees = pfolio.assets[symbol].fees;
          if (!(fees && fees.feeStructure.type === FeeType.FIXED)) return state;

          return {
            ...state,
            pfolios: {
              ...state.pfolios,
              [pfolio.id]: {
                ...pfolio,
                assets: {
                  ...pfolio.assets,
                  [symbol]: {
                    ...pfolio.assets[symbol],
                    fees: {
                      ...fees,
                      feeStructure: {
                        ...fees.feeStructure,
                        feeAmount: value || 0,
                      },
                    },
                  },
                },
                lastUpdatedAt: Date.now(),
              },
            },
          };
        }),
      setVariableFee: (payload) =>
        set((state) => {
          const pfolio = currentPortfolio(state);
          if (!pfolio) return state;
          if (
            !(pfolio.fees && pfolio.fees.feeStructure?.type === FeeType.VARIABLE)
          ) {
            return state;
          }

          return {
            ...state,
            pfolios: {
              ...state.pfolios,
              [pfolio.id]: {
                ...pfolio,
                fees: {
                  ...pfolio.fees,
                  feeStructure: {
                    ...pfolio.fees.feeStructure,
                    ...payload,
                  },
                },
                lastUpdatedAt: Date.now(),
              },
            },
          };
        }),
      setVariableFeeAsset: ({ symbol, ...rest }) =>
        set((state) => {
          const pfolio = currentPortfolio(state);
          if (!pfolio) return state;
          if (!symbol || !(symbol in pfolio.assets)) return state;

          const fees = pfolio.assets[symbol].fees;
          if (!(fees && fees.feeStructure.type === FeeType.VARIABLE)) return state;

          return {
            ...state,
            pfolios: {
              ...state.pfolios,
              [pfolio.id]: {
                ...pfolio,
                assets: {
                  ...pfolio.assets,
                  [symbol]: {
                    ...pfolio.assets[symbol],
                    fees: {
                      ...fees,
                      feeStructure: {
                        ...fees.feeStructure,
                        ...rest,
                      },
                    },
                  },
                },
                lastUpdatedAt: Date.now(),
              },
            },
          };
        }),
      clearBudget: () =>
        set((state) => {
          const pfolio = currentPortfolio(state);
          if (!pfolio) return state;

          return {
            ...state,
            pfolios: {
              ...state.pfolios,
              [pfolio.id]: {
                ...pfolio,
                budget: 0,
              },
            },
          };
        }),
      syncPortfoliosNow: async () => {
        const { pfolios, deletedPortfolios } = get();
        const payload = await syncPortfoliosAPI(pfolios, deletedPortfolios);
        set((state) => applySyncPortfoliosResult(state, payload as SyncPortfoliosPayload));
      },
    }),
    {
      name: PORTFOLIO_STORE_PERSIST_KEY,
      version: PORTFOLIO_STORE_PERSIST_VERSION,
      storage: createJSONStorage(() => portfolioStateStorage),
      partialize: (state) => ({
        selected: state.selected,
        pfolios: state.pfolios,
        deletedPortfolios: state.deletedPortfolios,
      }),
      migrate: (persistedState, persistedVersion) =>
        migratePortfolioStoreState(persistedState, persistedVersion),
    }
  )
);

export const useCurrentPortfolio = () => {
  return usePortfolioStore((state) => currentPortfolio(state));
};

export const getDefaultPortfolioName = () => {
  const defaultName = i18n.t("importStep.defaultPortfolioName");
  const defaultNameCount = Object.values(usePortfolioStore.getState().pfolios)
    .map((p) => p.name)
    .filter((name) => name.startsWith(defaultName)).length;

  return defaultNameCount > 0
    ? `${defaultName} ${defaultNameCount + 1}`
    : `${defaultName}`;
};

export const resetPortfolioStoreForTests = () => {
  usePortfolioStore.setState(initialPortfolioState());
  portfolioStoreStorage.removeItem(PORTFOLIO_STORE_PERSIST_KEY);
  portfolioStoreStorage.removeItem(REDUX_PERSIST_ROOT_KEY);
};

export const setPortfolioStoreStorageForTests = (storage: SyncStorage) => {
  portfolioStoreStorage = storage;
};

export {
  ACLASS,
  FeeType,
  aclassToString,
  currentPortfolio,
  feeTypeToString,
  getDefaultFees,
  getNewPortfolio,
  isWholeShares,
  parseAClass,
  parseFeeType,
  parseFees,
};
