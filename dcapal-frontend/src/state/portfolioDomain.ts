export const ACLASS = Object.freeze({
  UNDEFINED: 0,
  EQUITY: 10,
  CRYPTO: 20,
  CURRENCY: 30,
} as const);

export const FeeType = Object.freeze({
  ZERO_FEE: 10,
  FIXED: 20,
  VARIABLE: 30,
} as const);

export type PortfolioFees = {
  maxFeeImpact?: number | null;
  feeStructure: {
    type: number;
    feeAmount?: number | null;
    feeRate?: number | null;
    minFee?: number | null;
    maxFee?: number | null;
  };
} | null;

export type PortfolioAsset = {
  idx: number;
  symbol: string;
  name: string;
  aclass: number;
  baseCcy: string;
  provider: string;
  qty: number;
  targetWeight: number;
  price: number;
  amount: number;
  weight: number;
  fees: PortfolioFees;
};

export type Portfolio = {
  id: string;
  name: string;
  assets: Record<string, PortfolioAsset>;
  quoteCcy: string;
  nextIdx: number;
  totalAmount: number;
  budget: number;
  fees: PortfolioFees;
  lastPriceRefresh: number;
  lastUpdatedAt: number;
};

export type PortfolioStoreState = {
  selected: string | null;
  pfolios: Record<string, Portfolio>;
  deletedPortfolios: string[];
};

const getPortfolio = (
  id: string | null | undefined,
  pfolios: Record<string, Portfolio>
): Portfolio | null => {
  if (!id || !(id in pfolios)) return null;
  return pfolios[id];
};

export const currentPortfolio = (
  state: PortfolioStoreState | { pfolio?: PortfolioStoreState }
): Portfolio | null => {
  if ("pfolio" in state && state.pfolio) {
    return getPortfolio(state.pfolio.selected, state.pfolio.pfolios);
  }

  const local = state as PortfolioStoreState;
  return getPortfolio(local.selected, local.pfolios);
};

export const isWholeShares = (aclass: number): boolean => {
  switch (aclass) {
    case ACLASS.EQUITY:
      return true;
    default:
      return false;
  }
};

const aclassMap: Record<number, string> = {
  [ACLASS.UNDEFINED]: "UNDEFINED",
  [ACLASS.EQUITY]: "EQUITY",
  [ACLASS.CRYPTO]: "CRYPTO",
  [ACLASS.CURRENCY]: "CURRENCY",
};

export const aclassToString = (aclass: number): string => {
  return aclassMap[aclass] || "UNDEFINED";
};

export const parseAClass = (aclassStr: string): number => {
  if (aclassStr === "EQUITY") return ACLASS.EQUITY;
  if (aclassStr === "CRYPTO") return ACLASS.CRYPTO;
  if (aclassStr === "CURRENCY") return ACLASS.CURRENCY;
  return ACLASS.UNDEFINED;
};

export const feeTypeToString = (type: number): string => {
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

export const parseFeeType = (typeStr: string): number | null => {
  if (typeStr === "zeroFee") return FeeType.ZERO_FEE;
  if (typeStr === "fixed") return FeeType.FIXED;
  if (typeStr === "variable") return FeeType.VARIABLE;
  return null;
};

export const getDefaultFees = (type: number | null | undefined): PortfolioFees => {
  if (!type) {
    return null;
  }

  if (type === FeeType.ZERO_FEE) {
    return {
      feeStructure: {
        type: FeeType.ZERO_FEE,
      },
    };
  }

  if (type === FeeType.FIXED) {
    return {
      maxFeeImpact: null,
      feeStructure: {
        type: FeeType.FIXED,
        feeAmount: 0,
      },
    };
  }

  return {
    maxFeeImpact: null,
    feeStructure: {
      type: FeeType.VARIABLE,
      feeRate: 0,
      minFee: 0,
      maxFee: null,
    },
  };
};

export const parseFees = (fees: unknown): PortfolioFees => {
  if (!fees || typeof fees !== "object") return null;

  const parsedFees = fees as {
    maxFeeImpact?: number;
    feeStructure?: {
      type?: string;
      feeAmount?: number;
      feeRate?: number;
      minFee?: number;
      maxFee?: number | null;
    };
  };

  const feeType = parseFeeType(parsedFees.feeStructure?.type || "");
  if (!feeType) return null;

  const parsed = getDefaultFees(feeType);
  if (!parsed) return null;

  if (feeType === FeeType.ZERO_FEE) return parsed;

  if (feeType === FeeType.FIXED) {
    if (parsedFees.maxFeeImpact != null) {
      parsed.maxFeeImpact = parsedFees.maxFeeImpact;
    }
    if (parsedFees.feeStructure?.feeAmount != null) {
      parsed.feeStructure.feeAmount = parsedFees.feeStructure.feeAmount;
    }
    return parsed;
  }

  if (feeType === FeeType.VARIABLE) {
    if (parsedFees.maxFeeImpact != null) {
      parsed.maxFeeImpact = parsedFees.maxFeeImpact;
    }
    if (parsedFees.feeStructure?.feeRate != null) {
      parsed.feeStructure.feeRate = parsedFees.feeStructure.feeRate;
    }
    if (parsedFees.feeStructure?.minFee != null) {
      parsed.feeStructure.minFee = parsedFees.feeStructure.minFee;
    }
    if (parsedFees.feeStructure?.maxFee != null) {
      parsed.feeStructure.maxFee = parsedFees.feeStructure.maxFee;
    }
    return parsed;
  }

  return null;
};

export const getNewPortfolio = (): Portfolio => {
  return {
    id: crypto.randomUUID(),
    name: "",
    assets: {},
    quoteCcy: "eur",
    nextIdx: 0,
    totalAmount: 0,
    budget: 0,
    fees: getDefaultFees(FeeType.ZERO_FEE),
    lastPriceRefresh: Date.now(),
    lastUpdatedAt: Date.now(),
  };
};
