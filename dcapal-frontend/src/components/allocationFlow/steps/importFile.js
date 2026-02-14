import React, { useEffect, useMemo, useRef, useState } from "react";
import { useDispatch } from "react-redux";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { setAllocationFlowStep, setPfolioFile, Step } from "@app/appSlice";
import { getPriceForProvider } from "@/api/priceProviders";
import { useAppStore } from "@/state/appStore";
import { timeout } from "@utils/index.js";
import { Spinner } from "@components/spinner/spinner";
import {
  ACLASS,
  FeeType,
  addAsset,
  addPortfolio,
  getDefaultFees,
  getDefaultPortfolioName,
  getNewPortfolio,
  parseAClass,
  parseFees,
  selectPortfolio,
  setFeesAsset,
  setQty,
  setTargetWeight,
} from "@components/allocationFlow/portfolioSlice";

import IMPORT_PORTFOLIO_SVG from "@images/headers/import-portfolio.svg";

// Shared portfolio files contain user data, so prices are refreshed from the
// configured provider instead of trusting the serialized price value.
const importPfolio = async (id, pfolio, validCcys, dispatch) => {
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

  dispatch(addPortfolio({ pfolio: imported }));
  dispatch(selectPortfolio({ id: imported.id }));

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

    dispatch(
      addAsset({
        symbol: a.symbol,
        name: a.name,
        aclass: a.aclass ? parseAClass(a.aclass) : ACLASS.UNDEFINED,
        baseCcy: a.baseCcy,
        price: price,
        provider: a.provider,
      })
    );

    dispatch(setQty({ symbol: a.symbol, qty: Number(a.qty) }));
    dispatch(
      setTargetWeight({ symbol: a.symbol, weight: Number(a.targetWeight) })
    );

    const assetFees = (() => {
      if (a.fees != null && typeof a.fees === "object") {
        return parseFees(a.fees);
      } else {
        return null;
      }
    })();

    dispatch(setFeesAsset({ symbol: a.symbol, fees: assetFees }));
  }

  return true;
};

/** Imports a shared portfolio file into the allocation flow. */
export const ImportStep = () => {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState(false);
  const importStarted = useRef(false);
  const [pfolioId] = useState(crypto.randomUUID());
  const dispatch = useDispatch();
  const navigate = useNavigate();
  const { t } = useTranslation();

  const validCcys = useAppStore((state) => state.currencies);

  const pfolioFile = useAppStore((state) => state.pfolioFile);
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
        importPfolio(pfolioId, pfolio, validCcys, dispatch),
        timeout(1000),
      ]);

      if (success) {
        dispatch(setAllocationFlowStep({ step: Step.PORTFOLIO }));
      } else {
        setError(true);
      }

      dispatch(setPfolioFile({ file: "" }));
      setIsLoading(false);
    };

    runImport();
  }, [dispatch, pfolio, validCcys]);

  const onClickGoBack = () => {
    dispatch(setPfolioFile({ file: "" }));
    dispatch(setAllocationFlowStep({ step: Step.PORTFOLIOS }));
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
