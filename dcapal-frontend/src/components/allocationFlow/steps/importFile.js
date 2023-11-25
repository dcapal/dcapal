import React, { useEffect, useState } from "react";
import { useSelector } from "react-redux";
import { useDispatch } from "react-redux";
import { useNavigate } from "react-router-dom";
import { setAllocationFlowStep, setPfolioFile, Step } from "@app/appSlice";
import { getFetcher } from "@app/providers";
import { timeout } from "@utils/index.js";
import { Spinner } from "@components/spinner/spinner";
import {
  ACLASS,
  FeeType,
  addAsset,
  addPortfolio,
  getDefaultFees,
  getNewPortfolio,
  parseAClass,
  parseFeeType,
  selectPortfolio,
  setFeesAsset,
  setQty,
  setTargetWeight,
} from "@components/allocationFlow/portfolioSlice";

import { useTranslation } from "react-i18next";

import IMPORT_PORTFOLIO from "@images/headers/import-portfolio.svg";

const parseFees = (fees) => {
  if (!fees) return null;

  const parsed = {
    ...fees,
    feeStructure: {
      ...fees.feeStructure,
      type: parseFeeType(fees.feeStructure.type),
    },
  };

  if (!parsed.feeStructure.type) {
    return null;
  }

  return parsed;
};

const importPfolio = async (id, pfolio, validCcys, dispatch, t) => {
  const stopWithError = (...args) => {
    console.log(args);
  };

  if (!pfolio.quoteCcy) {
    stopWithError("[ImportStep] Missing 'quoteCcy' property");
    return false;
  }

  const imported = getNewPortfolio();
  imported.id = id;
  imported.name = pfolio.name || t("importStep.defaultPortfolioName");
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
    const price = await getFetcher(a.provider, validCcys)(
      a.symbol,
      pfolio.quoteCcy
    );
    if (!price) {
      console.warn(
        "[ImportStep] Failed to fetch price for",
        a.symbol,
        `(provider: ${pfolio.quoteCcy})`
      );
      continue;
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

    dispatch(setQty({ symbol: a.symbol, qty: a.qty }));
    dispatch(setTargetWeight({ symbol: a.symbol, weight: a.targetWeight }));

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

export const ImportStep = () => {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState(false);
  const [pfolioId] = useState(crypto.randomUUID());
  const dispatch = useDispatch();
  const navigate = useNavigate();
  const { t } = useTranslation();

  const pfolioFile = useSelector((state) => state.app.pfolioFile);
  const validCcys = useSelector((state) => state.app.currencies);

  useEffect(() => {
    return () => {
      dispatch(setPfolioFile({ file: "" }));
    };
  }, []);

  useEffect(() => {
    if (pfolioFile.length === 0) return;

    const pfolio = JSON.parse(pfolioFile);

    const runImport = async () => {
      const [success] = await Promise.all([
        // Pass `pfolioId` to `importPfolio` to overcome component re-render
        // issues that ended up adding the imported portfolio multiple times
        // with different UUIDs
        importPfolio(pfolioId, pfolio, validCcys, dispatch, t),
        timeout(1000),
      ]);

      if (success) {
        dispatch(setAllocationFlowStep({ step: Step.PORTFOLIO }));
      } else {
        setError(true);
      }

      setIsLoading(false);
    };

    runImport();
  }, [pfolioFile]);

  const onClickGoBack = () => {
    dispatch(setAllocationFlowStep({ step: Step.PORTFOLIOS }));
    navigate("/");
  };

  return (
    <div className="w-full flex flex-col items-center">
      <div className="px-6 py-10 flex flex-col grow justify-center items-center text-center gap-8">
        <img
          className="w-full px-4 sm:max-w-[20rem] pb-2"
          alt="Import Portfolio"
          src={IMPORT_PORTFOLIO}
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
          <>
            <h1 className="text-3xl font-bold">
              {t("importStep.importPortfolio")}
            </h1>
            <span className="flex flex-col gap-y-2 items-center font-light">
              <span className="text-4xl">⚠️</span>
              {t("importStep.ops")}...
            </span>
            <span
              className="font-medium underline cursor-pointer"
              onClick={onClickGoBack}
            >
              {t("common.goBack")}
            </span>
          </>
        )}
      </div>
    </div>
  );
};
