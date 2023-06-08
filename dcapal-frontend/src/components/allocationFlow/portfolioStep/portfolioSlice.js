import { createSlice } from "@reduxjs/toolkit";
import { roundPrice } from "../../../utils";

const updateWeight = (asset, totalAmount) => {
  const qty = asset.qty || 0;
  const price = asset.price || 0;
  const amount = qty * price;

  const weight = totalAmount > 0 ? amount / totalAmount : 0;

  asset.weight = weight * 100;
};

export const ACLASS = Object.freeze({
  UNDEFINED: 0,
  EQUITY: 10,
  CRYPTO: 20,
  CURRENCY: 30,
});

export const isWholeShares = (aclass) => {
  switch (aclass) {
    case ACLASS.EQUITY:
      return true;
    default:
      return false;
  }
};

export const aclassToString = (aclass) => {
  switch (aclass) {
    case ACLASS.UNDEFINED:
      return "UNDEFINED";
    case ACLASS.EQUITY:
      return "EQUITY";
    case ACLASS.CRYPTO:
      return "CRYPTO";
    case ACLASS.CURRENCY:
      return "CURRENCY";
    default:
      return "UNDEFINED";
  }
};

export const parseAClass = (aclassStr) => {
  if (aclassStr === "EQUITY") return ACLASS.EQUITY;
  if (aclassStr === "CRYPTO") return ACLASS.CRYPTO;
  if (aclassStr === "CURRENCY") return ACLASS.CURRENCY;

  return ACLASS.UNDEFINED;
};

export const portfolioSlice = createSlice({
  name: "portfolio",
  initialState: {
    assets: {},
    quoteCcy: "eur",
    nextIdx: 0,
    totalAmount: 0,
    budget: 0,
  },
  reducers: {
    addAsset: (state, action) => {
      const symbol = action.payload.symbol;
      if (symbol && symbol in state.assets) return;

      state.assets = {
        ...state.assets,
        [action.payload.symbol]: {
          idx: state.nextIdx,
          symbol: action.payload.symbol,
          name: action.payload.name,
          aclass: action.payload.aclass,
          baseCcy: action.payload.baseCcy,
          price: roundPrice(action.payload.price) || 0,
          provider: action.payload.provider,
          qty: 0,
          amount: 0,
          weight: 0,
          targetWeight: 0,
        },
      };
      state.nextIdx += 1;
    },
    removeAsset: (state, action) => {
      const symbol = action.payload.symbol;
      if (symbol in state.assets) {
        const asset = state.assets[symbol];
        const price = asset?.price || 0;
        const qty = asset?.qty || 0;

        state.totalAmount -= qty * price;
        delete state.assets[symbol];

        Object.values(state.assets).forEach((asset) => {
          updateWeight(asset, state.totalAmount);
        });
      }
    },
    setQty: (state, action) => {
      const asset = state.assets[action.payload.symbol];
      const price = asset?.price || 0;
      const qty = asset?.qty || 0;
      const newAmount = (action.payload.qty || 0) * price;

      state.totalAmount -= qty * price;
      state.totalAmount += newAmount;
      state.totalAmount = state.totalAmount;

      state.assets = {
        ...state.assets,
        [action.payload.symbol]: {
          ...state.assets[action.payload.symbol],
          qty: action.payload.qty || 0,
          amount: newAmount,
        },
      };

      Object.values(state.assets).forEach((asset) => {
        updateWeight(asset, state.totalAmount);
      });
    },
    setTargetWeight: (state, action) => {
      state.assets = {
        ...state.assets,
        [action.payload.symbol]: {
          ...state.assets[action.payload.symbol],
          targetWeight: action.payload.weight || 0,
        },
      };
    },
    setQuoteCurrency: (state, action) => {
      if (action.payload.quoteCcy && action.payload.quoteCcy.length > 0) {
        state.quoteCcy = action.payload.quoteCcy;
      }
    },
    setBudget: (state, action) => {
      if (action.payload.budget && action.payload.budget >= 0) {
        state.budget = action.payload.budget;
      }
    },
    clearPortfolio: (state) => {
      state.assets = {};
      state.nextIdx = 0;
      state.totalAmount = 0;
      state.budget = 0;
    },
    clearBudget: (state) => {
      state.budget = 0;
    },
  },
});

export const {
  addAsset,
  removeAsset,
  setQty,
  setTargetWeight,
  setQuoteCurrency,
  clearPortfolio,
  setBudget,
  clearBudget,
} = portfolioSlice.actions;

export default portfolioSlice.reducer;
