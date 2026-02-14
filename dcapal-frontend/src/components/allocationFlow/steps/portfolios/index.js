import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useDispatch } from "react-redux";
import { useAppStore } from "@/state/appStore";
import { getNewPortfolio, usePortfolioStore } from "@/state/portfolioStore";
import { PortfolioCard } from "./portfolioCard";
import { useNavigate } from "react-router-dom";
import { InputText } from "@components/core/inputText";
import { CcyGroup } from "@components/allocationFlow/steps/ccy/ccyGroup";
import {
  Step,
  setAllocationFlowStep,
  setPreferredCurrency,
} from "@app/appSlice";
import classNames from "classnames";
import { Button } from "@/components/ui/button";
import { supabase } from "@app/config";

export const PortfoliosStep = () => {
  const { t } = useTranslation();
  const pfolios = usePortfolioStore((state) => state.pfolios);
  const [showNewPfolio, setShowNewPfolio] = useState(
    Object.keys(pfolios).length === 0
  );

  const pfoliosCount = Object.keys(pfolios).length;

  useEffect(() => {
    if (pfoliosCount === 0) {
      setShowNewPfolio(true);
    }
  }, [pfoliosCount]);

  const onClickNewPortfolio = () => {
    setShowNewPfolio(true);
  };

  const title =
    pfoliosCount > 0
      ? t("portfoliosStep.myPortfolios")
      : t("portfoliosStep.newPortfolio");

  return (
    <div className="w-full flex flex-col items-center">
      <div className="mt-2 mb-8 text-3xl font-light">{title}</div>
      <div className="w-full flex flex-col gap-5 items-center">
        {Object.values(pfolios).map((p) => {
          return (
            <PortfolioCard
              key={p.id}
              id={p.id}
              name={p.name}
              ccy={p.quoteCcy}
              totalAmount={p.totalAmount}
              assets={Object.values(p.assets)}
            />
          );
        })}
      </div>
      {!showNewPfolio && (
        <Button
          variant="link"
          size="link"
          className="mt-5"
          onClick={onClickNewPortfolio}
        >
          {t("portfoliosStep.newPortfolio")}
        </Button>
      )}
      {showNewPfolio && pfoliosCount > 0 && (
        <p className="w-full mt-10 mb-3 text-2xl font-thin">
          {t("portfoliosStep.newPortfolio")}
        </p>
      )}
      {showNewPfolio && (
        <div className={classNames("w-full", { "pl-2": pfoliosCount > 0 })}>
          <NewPortfolioForm
            pfoliosCount={pfoliosCount}
            cancelCb={() => {
              setShowNewPfolio(false);
            }}
          />
        </div>
      )}
    </div>
  );
};

const priority = {
  usd: 10,
  eur: 20,
  gbp: 30,
  chf: 40,
  jpy: 50,
  cad: 60,
  aed: 70,
  aud: 80,
};

const sortCcy = (a, b) => {
  const [pA, pB] = [priority[a], priority[b]];
  if (pA && pB) return pA - pB;
  if (!pA && pB) return 1;
  if (pA && !pB) return -1;
  return a - b;
};

const NewPortfolioForm = ({ pfoliosCount, cancelCb }) => {
  const dispatch = useDispatch();
  const navigate = useNavigate();
  const { t } = useTranslation();
  const addPortfolio = usePortfolioStore((state) => state.addPortfolio);
  const selectPortfolio = usePortfolioStore((state) => state.selectPortfolio);
  const [name, setName] = useState("");

  const ccys = useAppStore((state) => state.currencies);
  const preferredCcy = useAppStore((state) => state.preferredCurrency);
  const sortedCcys = [...ccys].sort(sortCcy);

  const [selectedCcy, setSelectedCcy] = useState(
    preferredCcy ? preferredCcy : sortedCcys.length > 0 ? sortedCcys.at(0) : ""
  );

  useEffect(() => {
    setSelectedCcy(
      preferredCcy
        ? preferredCcy
        : sortedCcys.length > 0
          ? sortedCcys.at(0)
          : ""
    );
  }, [preferredCcy, ccys]);

  const onClickNext = () => {
    const pfolio = getNewPortfolio();
    pfolio.name = name;
    pfolio.quoteCcy = selectedCcy;

    addPortfolio({ pfolio: pfolio });
    selectPortfolio({ id: pfolio.id });
    dispatch(setPreferredCurrency({ ccy: selectedCcy }));
    dispatch(setAllocationFlowStep({ step: Step.PORTFOLIO }));
  };

  const onClickGoBack = () => {
    navigate("/");
  };

  const isFormValid = name && selectedCcy;

  return (
    <div className="w-full flex flex-col gap-4">
      <div className="w-full flex flex-col gap-1">
        <label className="text-sm font-light">{t("common.name")}</label>
        <InputText value={name} onChange={setName} isValid={true} />
      </div>
      <div className="w-full flex flex-col gap-2">
        <label className="text-sm font-light">{t("common.currency")}</label>
        <div>
          <CcyGroup
            ccys={sortedCcys}
            selected={selectedCcy}
            setSelected={setSelectedCcy}
          />
        </div>
      </div>
      <div className="w-full mt-6 flex justify-between items-center">
        <Button
          variant="link"
          size="link"
          onClick={pfoliosCount > 0 ? cancelCb : onClickGoBack}
        >
          {t("common.cancel")}
        </Button>
        <Button onClick={onClickNext} disabled={!isFormValid}>
          {t("common.next")}
        </Button>
      </div>
    </div>
  );
};
