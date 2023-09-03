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

export const FeeType = Object.freeze({
  ZERO_FEE: 10,
  FIXED: 20,
  VARIABLE: 30,
});

export const feeTypeToString = (type) => {
  switch (type) {
    case FeeType.ZERO_FEE:
      return "zeroFee";
    case FeeType.FIXED:
      return "fixed";
    case FeeType.VARIABLE:
      return "variable";
    default:
      return "undefined";
  }
};

export const parseFeeType = (typeStr) => {
  if (typeStr === "zeroFee") return FeeType.ZERO_FEE;
  if (typeStr === "fixed") return FeeType.FIXED;
  if (typeStr === "variable") return FeeType.VARIABLE;

  return null;
};

export const getDefaultFees = (type) => {
  if (!type) {
    return null;
  }

  if (type == FeeType.ZERO_FEE) {
    return {
      feeStructure: {
        type: FeeType.ZERO_FEE,
      },
    };
  } else if (type === FeeType.FIXED) {
    return {
      maxFeeImpact: null,
      feeStructure: {
        type: FeeType.FIXED,
        feeAmount: 0,
      },
    };
  } else {
    return {
      maxFeeImpact: null,
      feeStructure: {
        type: FeeType.VARIABLE,
        feeRate: 0,
        minFee: 0,
        maxFee: null,
      },
    };
  }
};

export const portfolioSlice = createSlice({
  name: "portfolio",
  initialState: {
    assets: {},
    quoteCcy: "eur",
    nextIdx: 0,
    totalAmount: 0,
    budget: 0,
    fees: getDefaultFees(FeeType.ZERO_FEE),
    lastPriceRefresh: Date.now(),
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
          fees: null,
        },
      };
      state.nextIdx += 1;

      if (Object.keys(state.assets).length === 1) {
        state.lastPriceRefresh = new Date();
      }
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
    setPrice: (state, action) => {
      const { symbol, price } = action.payload;
      if (!symbol || !(symbol in state.assets)) {
        return;
      }

      const asset = state.assets[action.payload.symbol];

      // Refresh amounts
      const newAmount = (asset.qty || 0) * price;
      state.totalAmount -= asset.qty * price;
      state.totalAmount += newAmount;
      state.totalAmount = state.totalAmount;

      // Update asset info
      asset.price = price;
      asset.amount = newAmount;

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
    setRefreshTime: (state, action) => {
      if (action.payload.time) {
        state.lastPriceRefresh = action.payload.time;
      }
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
    setFees: (state, action) => {
      if (action.payload.fees) {
        state.fees = action.payload.fees;
      }
    },
    setFeesAsset: (state, action) => {
      const { fees, symbol } = action.payload;
      if (!symbol || !(symbol in state.assets)) {
        return;
      }

      state.assets[symbol].fees = fees;
    },
    setFeeType: (state, action) => {
      if (!action.payload.type) {
        state.fees = getDefaultFees(FeeType.ZERO_FEE);
        return;
      }

      if (!state.fees?.feeStructure?.type) {
        state.fees = getDefaultFees(action.payload.type);
        return;
      }

      if (action.payload.type !== state.fees.feeStructure.type) {
        state.fees = {
          ...state.fees,
          feeStructure: getDefaultFees(action.payload.type).feeStructure,
        };
      }
    },
    setFeeTypeAsset: (state, action) => {
      const { type, symbol } = action.payload;
      if (!symbol || !(symbol in state.assets)) {
        return;
      }

      if (!type) {
        state.assets[symbol].fees = null;
        return;
      }

      const asset = state.assets[symbol];

      if (!asset.fees?.feeStructure.type) {
        asset.fees = getDefaultFees(type);
        return;
      }

      if (type !== asset.fees.feeStructure.type) {
        asset.fees = {
          ...asset.fees,
          feeStructure: getDefaultFees(type).feeStructure,
        };
      }
    },
    setMaxFeeImpact: (state, action) => {
      if (state.fees) {
        state.fees = {
          ...state.fees,
          maxFeeImpact: action.payload.value,
        };
      }
    },
    setMaxFeeImpactAsset: (state, action) => {
      const { value, symbol } = action.payload;
      if (!symbol || !(symbol in state.assets)) {
        return;
      }

      const fees = state.assets[symbol].fees;
      if (fees) {
        state.assets[symbol].fees = {
          ...fees,
          maxFeeImpact: value,
        };
      }
    },
    setFixedFeeAmount: (state, action) => {
      if (state.fees && state.fees.feeStructure?.type === FeeType.FIXED) {
        state.fees = {
          ...state.fees,
          feeStructure: {
            ...state.fees.feeStructure,
            feeAmount: action.payload.value,
          },
        };
      }
    },
    setFixedFeeAmountAsset: (state, action) => {
      const { value, symbol } = action.payload;
      if (!symbol || !(symbol in state.assets)) {
        return;
      }

      const fees = state.assets[symbol].fees;
      if (fees && fees.feeStructure.type === FeeType.FIXED) {
        state.assets[symbol].fees = {
          ...fees,
          feeStructure: {
            ...fees.feeStructure,
            feeAmount: value,
          },
        };
      }
    },
    setVariableFee: (state, action) => {
      if (state.fees && state.fees.feeStructure?.type === FeeType.VARIABLE) {
        state.fees = {
          ...state.fees,
          feeStructure: {
            ...state.fees.feeStructure,
            ...action.payload,
          },
        };
      }
    },
    setVariableFeeAsset: (state, action) => {
      const { symbol, ...rest } = action.payload;
      if (!symbol || !(symbol in state.assets)) {
        return;
      }

      const fees = state.assets[symbol].fees;
      if (fees && fees.feeStructure.type === FeeType.VARIABLE) {
        state.assets[symbol].fees = {
          ...fees,
          feeStructure: {
            ...fees.feeStructure,
            ...rest,
          },
        };
      }
    },
    clearPortfolio: (state) => {
      state.assets = {};
      state.nextIdx = 0;
      state.totalAmount = 0;
      state.budget = 0;
      state.fees = getDefaultFees(FeeType.ZERO_FEE);
      state.lastPriceRefresh = Date.now();
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
  setPrice,
  setTargetWeight,
  setRefreshTime,
  setQuoteCurrency,
  setBudget,
  setFees,
  setFeesAsset,
  setFeeType,
  setFeeTypeAsset,
  setMaxFeeImpact,
  setMaxFeeImpactAsset,
  setFixedFeeAmount,
  setFixedFeeAmountAsset,
  setVariableFee,
  setVariableFeeAsset,
  clearPortfolio,
  clearBudget,
} = portfolioSlice.actions;

export default portfolioSlice.reducer;
