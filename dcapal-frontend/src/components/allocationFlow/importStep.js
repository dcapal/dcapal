import React, { useEffect, useState } from "react";
import { useSelector } from "react-redux";
import { useDispatch } from "react-redux";
import { useNavigate } from "react-router-dom";
import { setAllocationFlowStep, setPfolioFile, Step } from "../../app/appSlice";
import { getFetcher } from "../../app/providers";
import { timeout } from "../../utils";
import { Spinner } from "../spinner/spinner";
import {
  ACLASS,
  FeeType,
  addAsset,
  clearPortfolio,
  getDefaultFees,
  parseAClass,
  parseFeeType,
  setFees,
  setFeesAsset,
  setQty,
  setQuoteCurrency,
  setTargetWeight,
} from "./portfolioStep/portfolioSlice";

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

const importPfolio = async (pfolio, validCcys, dispatch) => {
  const stopWithError = (...args) => {
    console.log(args);
  };

  if (!pfolio.quoteCcy) {
    stopWithError("[ImportStep] Missing 'quoteCcy' property");
    return false;
  }

  dispatch(clearPortfolio());
  dispatch(setQuoteCurrency({ quoteCcy: pfolio.quoteCcy }));

  const fees = (() => {
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

  dispatch(setFees({ fees: fees }));

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
  const dispatch = useDispatch();
  const navigate = useNavigate();

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
        importPfolio(pfolio, validCcys, dispatch),
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
    dispatch(clearPortfolio({}));
    dispatch(setAllocationFlowStep({ step: Step.CCY }));
    navigate("/");
  };

  return (
    <div className="w-full h-full flex flex-col items-center">
      <div className="px-6 py-10 flex flex-col grow justify-center items-center text-center gap-8">
        <img
          className="w-full px-4 sm:max-w-[20rem] pb-2"
          alt="Import Portfolio"
          src={IMPORT_PORTFOLIO}
        />
        {isLoading && (
          <>
            <h1 className="text-3xl font-bold">Import Portfolio</h1>
            <span className="flex flex-col gap-y-2 items-center font-light">
              <p>Just a sec! Fetching fresh data for your portfolio...</p>
            </span>
            <Spinner />
          </>
        )}
        {!isLoading && error && (
          <>
            <h1 className="text-3xl font-bold">Import Portfolio</h1>
            <span className="flex flex-col gap-y-2 items-center font-light">
              <span className="text-4xl">⚠️</span> Oops! This is embarassing...
            </span>
            <span
              className="font-medium underline cursor-pointer"
              onClick={onClickGoBack}
            >
              Go back
            </span>
          </>
        )}
      </div>
    </div>
  );
};
