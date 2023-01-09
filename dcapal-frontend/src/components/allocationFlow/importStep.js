import React, { useEffect, useState } from "react";
import { useSelector } from "react-redux";
import { useDispatch } from "react-redux";
import { useNavigate } from "react-router-dom";
import { setAllocationFlowStep, setPfolioFile, Step } from "../../app/appSlice";
import { getFetcher } from "../../app/providers";
import { timeout } from "../../utils";
import { Spinner } from "../spinner/spinner";
import {
  addAsset,
  setQty,
  setQuoteCurrency,
  setTargetWeight,
} from "./portfolioStep/portfolioSlice";

const importPfolio = async (pfolio, setError, setIsLoading, dispatch) => {
  const stopWithError = (...args) => {
    console.log(args);
    setError(true);
    setIsLoading(false);
  };

  if (!pfolio.quoteCcy) {
    stopWithError("[ImportStep] Missing 'quoteCcy' property");
    return;
  }

  dispatch(setQuoteCurrency({ quoteCcy: pfolio.quoteCcy }));

  if (!pfolio.assets || !Array.isArray(pfolio.assets)) {
    stopWithError("[ImportStep] Missing 'assets' property");
    return;
  }

  for (const a of pfolio.assets) {
    const price = await getFetcher(a.provider)(a.symbol, pfolio.quoteCcy);
    if (!price) {
      console.log(
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
        baseCcy: a.baseCcy,
        price: price,
        provider: a.provider,
      })
    );
    dispatch(setQty({ symbol: a.symbol, qty: a.qty }));
    dispatch(setTargetWeight({ symbol: a.symbol, weight: a.targetWeight }));
  }
};

export const ImportStep = () => {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState(false);
  const dispatch = useDispatch();
  const navigate = useNavigate();

  const pfolioFile = useSelector((state) => state.app.pfolioFile);

  useEffect(() => {
    return () => {
      dispatch(setPfolioFile({ file: "" }));
    };
  }, []);

  useEffect(() => {
    if (pfolioFile.length === 0) return;

    const pfolio = JSON.parse(pfolioFile);

    const runImport = async () => {
      await Promise.all([
        importPfolio(pfolio, setError, setIsLoading, dispatch),
        timeout(1000),
      ]);

      setIsLoading(false);
      dispatch(setAllocationFlowStep({ step: Step.PORTFOLIO }));
    };

    runImport();
  }, [pfolioFile]);

  const onClickGoBack = () => {
    navigate("/");
  };

  return (
    <div className="w-full h-full flex flex-col items-center">
      {isLoading && (
        <>
          <div className="mt-2 mb-8 text-3xl font-light">
            <span className="text-4xl">📡</span>Just a sec! Fetching fresh data
            for your portfolio
          </div>
          <Spinner />
        </>
      )}
      {!isLoading && error && (
        <>
          <div className="mt-2 mb-8 text-3xl font-light">
            <span className="text-4xl">⚠️</span> Oops! This is embarassing...
          </div>
          <span
            className="font-medium underline cursor-pointer"
            onClick={onClickGoBack}
          >
            Go back
          </span>
        </>
      )}
    </div>
  );
};
