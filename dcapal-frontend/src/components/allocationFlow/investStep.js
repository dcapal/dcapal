import React, { useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import { useCollapse } from "react-collapsed";
import { setAllocationFlowStep, Step } from "../../app/appSlice";
import { InputNumber, InputNumberType } from "../core/inputNumber";
import { setBudget } from "./portfolioStep/portfolioSlice";
import classNames from "classnames";
import { Trans, useTranslation } from "react-i18next";

export const InvestStep = ({
  useTaxEfficient,
  useWholeShares,
  setUseTaxEfficient,
  setUseWholeShares,
}) => {
  const [cash, setCash] = useState(0);
  const dispatch = useDispatch();
  const { t } = useTranslation();

  const quoteCcy = useSelector((state) => state.pfolio.quoteCcy);
  const totalAmount = useSelector((state) => state.pfolio.totalAmount);

  const { getCollapseProps, getToggleProps, isExpanded } = useCollapse();

  const onClickTaxEfficient = (e) => {
    setUseTaxEfficient(!useTaxEfficient);
  };

  const onClickWholeShares = (e) => {
    setUseWholeShares(!useWholeShares);
  };

  const onClickGoBack = () => {
    dispatch(setAllocationFlowStep({ step: Step.PORTFOLIO }));
  };

  const onClickRunAllocation = () => {
    dispatch(setBudget({ budget: cash }));
    dispatch(setAllocationFlowStep({ step: Step.END }));
  };

  const isRunAllocationDisabled = cash + totalAmount <= 0;

  return (
    <div className="w-full h-full flex flex-col items-center">
      <div className="mt-2 mb-8 text-3xl font-light">
        {t("investStep.howMuchAllocate")}
      </div>
      <div className="w-full flex justify-center items-end mb-20">
        <div className="w-full">
          <InputNumber
            textSize="4rem"
            textAlign="text-right"
            type={InputNumberType.DECIMAL}
            value={cash}
            onChange={setCash}
            isValid={true}
            min={0}
            leadingNone={true}
          />
        </div>
        <div className="ml-2 pb-2 text-2xl font-light uppercase">
          {quoteCcy}
        </div>
      </div>
      <div className="w-full flex flex-col gap-1 justify-start">
        <div
          className="w-full flex items-center cursor-pointer"
          onClick={onClickTaxEfficient}
        >
          <input
            id="tax-efficient-checkbox"
            type="checkbox"
            className="w-4 h-4 accent-neutral-500 cursor-pointer"
            checked={useTaxEfficient}
            onChange={onClickTaxEfficient}
          />
          <label
            htmlFor="#tax-efficient-checkbox"
            className="ml-2 cursor-pointer select-none"
          >
            <Trans
              i18nKey="investStep.taxEfficientAlgorithm"
              values={{
                tax: t("investStep.taxEfficient"),
              }}
              components={[<span className="font-medium" />]}
            />
          </label>
        </div>
        <p className="text-sm font-light">
          <Trans
            i18nKey="investStep.taxEfficientInfo"
            values={{
              tax: t("investStep.taxEfficient"),
            }}
            components={[<span className="italic" />]}
          />
        </p>
      </div>
      <div className="w-full mt-6 flex flex-col gap-3">
        <div
          className="flex gap-1 items-center font-light text-xs"
          {...getToggleProps()}
        >
          <span
            className={classNames("transition-transform", {
              "rotate-90": isExpanded,
            })}
          >
            {">"}
          </span>
          <span>{t("investStep.advanced")}</span>
        </div>
        <div
          className="w-full pl-6 flex flex-col gap-1 justify-start text-sm"
          {...getCollapseProps()}
        >
          <div
            className="w-full flex items-center cursor-pointer"
            onClick={onClickWholeShares}
          >
            <input
              id="tax-efficient-checkbox"
              type="checkbox"
              className="w-4 h-4 accent-neutral-500 cursor-pointer"
              checked={useWholeShares}
              onChange={onClickWholeShares}
            />
            <label
              htmlFor="#tax-efficient-checkbox"
              className="ml-2 cursor-pointer select-none"
            >
              <span className="font-medium">{t("investStep.doNotSplit")}</span>
              {t("investStep.wholeShares")}
            </label>
          </div>
          <p className="text-sm font-light">
            <Trans
              i18nKey="investStep.doNotSplitInfo"
              values={{
                message:
                  t("investStep.doNotSplit") + t("investStep.wholeShares"),
              }}
              components={[<span className="italic" />]}
            />
          </p>
        </div>
      </div>
      <div className="w-full mt-6 flex items-center justify-between">
        <span
          className="font-medium underline cursor-pointer"
          onClick={onClickGoBack}
        >
          {t("common.goBack")}{" "}
        </span>
        <button
          className="px-3 pt-1.5 pb-2 flex justify-center items-center bg-neutral-500 hover:bg-neutral-600 active:bg-neutral-800 text-white text-lg rounded-md shadow-md disabled:pointer-events-none disabled:opacity-60"
          onClick={onClickRunAllocation}
          disabled={isRunAllocationDisabled}
        >
          {t("investStep.runAllocation")}{" "}
        </button>
      </div>
    </div>
  );
};
