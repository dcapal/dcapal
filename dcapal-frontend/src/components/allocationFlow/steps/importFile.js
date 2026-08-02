import React, { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { Step, useAppStore } from "@/state/appStore";
import { getPriceForProvider } from "@/api/priceProviders";
import { timeout } from "@utils/index.js";
import { Spinner } from "@components/spinner/spinner";
import {
  ACLASS,
  FeeType,
  getDefaultFees,
  getDefaultPortfolioName,
  parseAClass,
  parseFees,
  getNewPortfolio,
  usePortfolioStore,
} from "@/state/portfolioStore";

import IMPORT_PORTFOLIO_SVG from "@images/headers/import-portfolio.svg";

const importPfolio = async (id, pfolio, validCcys, portfolioActions) => {
  const {
    addAsset,
    addPortfolio,
    selectPortfolio,
    setFeesAsset,
    setQty,
    setTargetWeight,
  } = portfolioActions;

  const stopWithError = (...args) => {
    console.log(args);
  };

  if (!pfolio.quoteCcy) {
    stopWithError("[ImportStep] Missing 'quoteCcy' property");
    return false;
  }

  const imported = getNewPortfolio();
  imported.id = id;
  imported.name = pfolio.name;
  imported.quoteCcy = pfolio.quoteCcy;

  imported.fees = (() => {
    if (pfolio.fees != null && typeof pfolio.fees === "object") {
      return parseFees(pfolio.fees) || getDefaultFees(FeeType.ZERO_FEE);
    } else {
      return getDefaultFees(FeeType.ZERO_FEE);
    }
  })();

  if (!pfolio.assets || !Array.isArray(pfolio.assets)) {
    stopWithError("[ImportStep] Missing 'assets' property");
    return false;
  }

  addPortfolio({ pfolio: imported });
  selectPortfolio({ id: imported.id });

  for (const a of pfolio.assets) {
    const price = await getPriceForProvider(
      a.provider,
      validCcys,
      a.symbol,
      pfolio.quoteCcy
    );
    if (price == null) {
      console.warn(
        "[ImportStep] Failed to fetch price for",
        a.symbol,
        `(provider=${a.provider} quoteCcy=${pfolio.quoteCcy})`
      );
      return false;
    }

    addAsset({
      symbol: a.symbol,
      name: a.name,
      aclass: a.aclass ? parseAClass(a.aclass) : ACLASS.UNDEFINED,
      baseCcy: a.baseCcy,
      price: price,
      provider: a.provider,
    });

    setQty({ symbol: a.symbol, qty: a.qty });
    setTargetWeight({ symbol: a.symbol, weight: a.targetWeight });

    const assetFees = (() => {
      if (a.fees != null && typeof a.fees === "object") {
        return parseFees(a.fees);
      } else {
        return null;
      }
    })();

    setFeesAsset({ symbol: a.symbol, fees: assetFees });
  }

  return true;
};

/** Imports a shared portfolio file into the allocation flow. */
export const ImportStep = () => {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState(false);
  const importStarted = useRef(false);
  const [pfolioId] = useState(crypto.randomUUID());
  const navigate = useNavigate();
  const { t } = useTranslation();
  const addAsset = usePortfolioStore((state) => state.addAsset);
  const addPortfolio = usePortfolioStore((state) => state.addPortfolio);
  const selectPortfolio = usePortfolioStore((state) => state.selectPortfolio);
  const setFeesAsset = usePortfolioStore((state) => state.setFeesAsset);
  const setQty = usePortfolioStore((state) => state.setQty);
  const setTargetWeight = usePortfolioStore((state) => state.setTargetWeight);

  const validCcys = useAppStore((state) => state.currencies);
  const pfolioFile = useAppStore((state) => state.pfolioFile);
  const setAllocationFlowStep = useAppStore(
    (state) => state.setAllocationFlowStep
  );
  const setPfolioFile = useAppStore((state) => state.setPfolioFile);
  const pfolio = useMemo(() => {
    const parsed = pfolioFile ? JSON.parse(pfolioFile) : {};
    if (Object.keys(parsed).length > 0) {
      parsed.name = parsed.name ?? getDefaultPortfolioName();
    }
    return parsed;
  }, [pfolioFile]);

  useEffect(() => {
    if (
      Object.keys(pfolio).length === 0 ||
      validCcys.length === 0 ||
      importStarted.current
    ) {
      return;
    }

    importStarted.current = true;

    const runImport = async () => {
      const [success] = await Promise.all([
        // Pass `pfolioId` to `importPfolio` to overcome component re-render
        // issues that ended up adding the imported portfolio multiple times
        // with different UUIDs
        importPfolio(pfolioId, pfolio, validCcys, {
          addAsset,
          addPortfolio,
          selectPortfolio,
          setFeesAsset,
          setQty,
          setTargetWeight,
        }),
        timeout(1000),
      ]);

      if (success) {
        setAllocationFlowStep({ step: Step.PORTFOLIO });
      } else {
        setError(true);
      }

      setPfolioFile({ file: "" });
      setIsLoading(false);
    };

    runImport();
  }, [pfolio, pfolioId, setAllocationFlowStep, setPfolioFile, validCcys]);

  const onClickGoBack = () => {
    setPfolioFile({ file: "" });
    setAllocationFlowStep({ step: Step.PORTFOLIOS });
    navigate("/");
  };

  return (
    <div className="w-full flex flex-col items-center">
      <div className="px-6 py-10 flex flex-col grow justify-center items-center text-center gap-8">
        <img
          className="w-full px-4 sm:max-w-[20rem] pb-2"
          alt="Import Portfolio"
          src={IMPORT_PORTFOLIO_SVG}
        />
        {isLoading && (
          <>
            <h1 className="text-3xl font-bold">
              {t("importStep.importPortfolio")}
            </h1>
            <span className="flex flex-col gap-y-2 items-center font-light">
              <p>{t("importStep.fetchData")}...</p>
            </span>
            <Spinner />
          </>
        )}
        {!isLoading && error && (
          <div data-testid="import-error" className="contents">
            <h1 className="text-3xl font-bold">
              {t("importStep.importPortfolio")}
            </h1>
            <span className="flex flex-col gap-y-2 items-center font-light">
              <span className="text-4xl">⚠️</span>
              {t("importStep.ops")}...
            </span>
            <button
              type="button"
              className="font-medium underline cursor-pointer"
              onClick={onClickGoBack}
            >
              {t("common.goBack")}
            </button>
          </div>
        )}
      </div>
    </div>
  );
};
