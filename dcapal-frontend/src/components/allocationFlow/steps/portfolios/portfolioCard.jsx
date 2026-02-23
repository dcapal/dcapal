import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { InputText } from "@components/core/inputText";
import { useDispatch } from "react-redux";
import {
  deletePortfolio,
  duplicatePortfolio,
  renamePortfolio,
  selectPortfolio,
} from "@components/allocationFlow/portfolioSlice";
import { Step, setAllocationFlowStep } from "@app/appSlice";

import EDIT_SVG from "@images/icons/edit.svg";
import CLOSE_SVG from "@images/icons/close-menu.svg";
import { Button } from "@/components/ui/button";
import { useSyncPortfolios } from "@hooks/useSyncPortfolios";
import classNames from "classnames";

const orderByWeightDesc = (a, b) => b.weight - a.weight;

const weightFmt = {
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
};

const amtFmt = {
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
};

const CardState = Object.freeze({
  VIEW: 0,
  EDIT: 10,
});

const computePortfolioGain = (assets) => {
  let totalCost = 0;
  let totalValue = 0;

  assets.forEach((a) => {
    if (a.qty > 0) {
      const effectiveABP = a.averageBuyPrice || a.price;
      totalCost += effectiveABP * a.qty;
      totalValue += a.price * a.qty;
    }
  });

  if (totalCost === 0) return null;

  return ((totalValue - totalCost) / totalCost) * 100;
};

export const PortfolioCard = ({ id, name, ccy, totalAmount, assets }) => {
  const [state, setState] = useState(CardState.VIEW);
  const [newName, setNewName] = useState(name);

  const dispatch = useDispatch();
  const { t, i18n } = useTranslation();

  const onClickPortfolio = () => {
    dispatch(selectPortfolio({ id: id }));
    dispatch(setAllocationFlowStep({ step: Step.PORTFOLIO }));
  };

  const { syncNow, isAuthenticated } = useSyncPortfolios();

  const onClickEdit = () => {
    if (state === CardState.VIEW) {
      setState(CardState.EDIT);
    } else {
      setNewName(name);
      setState(CardState.VIEW);
    }
  };

  const onClickSave = () => {
    dispatch(renamePortfolio({ id: id, name: newName }));
    setState(CardState.VIEW);
    if (isAuthenticated) syncNow();
  };

  const onClickDelete = () => {
    dispatch(deletePortfolio({ id: id }));
    if (isAuthenticated) syncNow();
  };

  const onClickDuplicate = () => {
    dispatch(duplicatePortfolio({ id: id }));
    onClickEdit();
    if (isAuthenticated) syncNow();
  };

  const editIcon = state === CardState.VIEW ? EDIT_SVG : CLOSE_SVG;

  const portfolioGain = computePortfolioGain(assets);
  const hasAssetsWithQty = assets.some((a) => a.qty > 0);

  return (
    <div className="relative w-full max-w-[36rem] flex flex-col px-3 pt-2 pb-3 shadow-md ring-1 ring-black/5 rounded-md bg-white cursor-pointer hover:ring-2 hover:ring-neutral-400 active:ring-2 active:ring-neutral-500 focus-visible:ring-2 focus-visible:ring-neutral-500">
      <div
        className="absolute flex justify-center items-center w-8 h-8 -right-4 -top-3.5 rounded-full bg-neutral-500 hover:bg-neutral-600 active:bg-neutral-800 cursor-pointer"
        onClick={onClickEdit}
      >
        <img className="w-3/5" alt="Edit portfolio" src={editIcon} />
      </div>
      {state === CardState.VIEW && (
        <div className="w-full flex flex-col" onClick={onClickPortfolio}>
          <div className="text-lg truncate font-medium" title={name}>
            {name}
          </div>
          <div className="flex items-center gap-2">
            <p className="text-sm uppercase">
              {totalAmount.toLocaleString(i18n.language, amtFmt)} {ccy}
            </p>
            {hasAssetsWithQty && portfolioGain !== null && (
              <span
                className={classNames("text-sm font-semibold", {
                  "text-green-600": portfolioGain > 0,
                  "text-red-600": portfolioGain < 0,
                  "text-neutral-500": portfolioGain === 0,
                })}
              >
                {portfolioGain > 0 ? "+" : ""}
                {portfolioGain.toLocaleString(i18n.language, {
                  minimumFractionDigits: 2,
                  maximumFractionDigits: 2,
                })}
                %
              </span>
            )}
          </div>
          {assets.length > 0 && (
            <div className="w-full flex flex-col gap-1 max-h-32 mt-2 overflow-auto">
              {assets.sort(orderByWeightDesc).map((a) => (
                <div className="flex justify-between" key={a.symbol}>
                  <label className="min-w-0 font-light truncate">
                    {a.name}
                  </label>
                  <p className="whitespace-nowrap">
                    {a.weight.toLocaleString(i18n.language, weightFmt)} %
                  </p>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
      {state === CardState.EDIT && (
        <div className="w-full flex flex-col">
          <div className="flex flex-col gap-1">
            <p className="pl-1 text-sm font-light">{t("common.name")}</p>
            <InputText
              value={newName}
              onChange={setNewName}
              isValid={newName}
            />
          </div>
          <div className="w-full mt-5 flex justify-between items-center">
            <Button variant="link" size="sm" onClick={onClickDelete}>
              {t("portfoliosStep.deletePortfolio")}
            </Button>
            <div className="flex gap-1">
              <Button
                variant="outline"
                size="sm"
                disabled={!newName}
                onClick={onClickDuplicate}
              >
                {t("common.duplicate")}
              </Button>
              <Button size="sm" disabled={!newName} onClick={onClickSave}>
                {t("common.save")}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
