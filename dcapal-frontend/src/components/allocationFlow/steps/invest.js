import React, { useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import { setAllocationFlowStep, Step } from "@app/appSlice";
import { InputNumber, InputNumberType } from "@components/core/inputNumber";
import {
  currentPortfolio,
  setBudget,
} from "@components/allocationFlow/portfolioSlice";
import { Trans, useTranslation } from "react-i18next";
import { useComputeWorker } from "@/compute";
import { Button } from "@/components/ui/button";
import { useSuggestAmountToInvest } from "./useInvestCompute";

const amtDecimals = 2;

export const InvestStep = ({
  useTaxEfficient,
  useWholeShares,
  useAllBudget,
  setUseTaxEfficient,
  setUseWholeShares,
  setUseAllBudget,
}) => {
  const [cash, setCash] = useState(0);
  const dispatch = useDispatch();
  const { t } = useTranslation();
  const [workerStatus, worker] = useComputeWorker();
  const { isReady: isWorkerReady, isLoading: isWorkerLoading } = workerStatus;

  const quoteCcy = useSelector((state) => currentPortfolio(state).quoteCcy);
  const totalAmount = useSelector(
    (state) => currentPortfolio(state).totalAmount
  );
  const assets = useSelector((state) => currentPortfolio(state).assets);
  const { solution, isLoading } = useSuggestAmountToInvest({
    assets,
    totalAmount,
    isWorkerReady,
    worker,
  });

  const handleButtonClick = () => {
    setCash(+Number(solution).toFixed(amtDecimals));
  };

  const onClickTaxEfficient = () => {
    setUseTaxEfficient(!useTaxEfficient);
  };

  const onClickWholeShares = () => {
    setUseWholeShares(!useWholeShares);
  };

  const onClickUseAllBudget = () => {
    setUseAllBudget(!useAllBudget);
  };

  const onClickGoBack = () => {
    dispatch(setAllocationFlowStep({ step: Step.PORTFOLIO }));
  };

  const onClickRunAllocation = () => {
    dispatch(setBudget({ budget: cash }));
    dispatch(setAllocationFlowStep({ step: Step.END }));
  };

  const isRunAllocationDisabled = cash + totalAmount <= 0 || !isWorkerReady;

  let i18nKey = "";

  if (totalAmount !== 0) {
    if (isWorkerLoading || !isWorkerReady || (solution === null && isLoading)) {
      i18nKey = "investStep.loading";
    } else if (Number(solution) !== 0) {
      i18nKey = "investStep.youShouldAllocateAmount";
    } else {
      i18nKey = "investStep.youAlreadyReachedTargetAllocation";
    }
  }

  return (
    <div className="w-full flex flex-col items-center">
      <div className="mt-2 mb-8 text-3xl font-light">
        {t("investStep.howMuchAllocate")}
      </div>
      <div className="w-full flex justify-center items-end">
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
            dataTestId="invest.cash.input"
          />
        </div>
        <div className="ml-2 pb-2 text-2xl font-light uppercase">
          {quoteCcy}
        </div>
      </div>
      <div className="mt-2 text-xl font-light" data-testid="invest.recommendation.text">
        <Trans
          i18nKey={i18nKey}
          values={{
            solution:
              Number(solution) !== 0
                ? (+Number(solution).toFixed(amtDecimals)).toLocaleString()
                : solution,
            quoteCcy: String(quoteCcy),
          }}
          components={[
            <span className="font-medium" />,
            <span className="uppercase" />,
          ]}
        />
        {Number(solution) !== 0 ? (
          <>
            <Button
              variant="link"
              className="p-0 pr-1"
              onClick={handleButtonClick}
            >
              {t("investStep.clickHere")}
            </Button>
            <span>{t("investStep.toInsertAmount")}</span>
          </>
        ) : null}
      </div>

      <div className="w-full flex flex-col gap-4 mt-20">
        <p className="text"></p>
        <div className="w-full flex flex-col gap-1 justify-start">
          <div
            className="w-full flex items-center cursor-pointer"
            onClick={onClickTaxEfficient}
          >
            <input
              data-testid="invest.taxEfficient.checkbox"
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
                components={[<span className="font-medium capitalize" />]}
              />
            </label>
          </div>
          <p className="text-sm font-light">
            <Trans
              i18nKey="investStep.taxEfficientInfo"
              values={{
                tax: t("investStep.taxEfficient"),
              }}
              components={[<span className="italic capitalize" />]}
            />
          </p>
        </div>
        <div className="w-full flex flex-col gap-1 justify-start">
          <div
            className="w-full flex items-center cursor-pointer"
            onClick={onClickUseAllBudget}
          >
            <input
              id="use-all-budget-checkbox"
              type="checkbox"
              className="w-4 h-4 accent-neutral-500 cursor-pointer"
              checked={useAllBudget}
              onChange={onClickUseAllBudget}
            />
            <label
              htmlFor="#use-all-budget-checkbox"
              className="ml-2 cursor-pointer select-none"
            >
              {t("investStep.allocate")} <span className="font-medium capitalize">{t("investStep.allBudget")}</span>
            </label>
          </div>
          <p className="text-sm font-light">
            <Trans
              i18nKey="investStep.allocateAllBudgetInfo"
              values={{
                message:
                  t("investStep.allocate") + " " + t("investStep.allBudget"),
              }}
              components={[<span className="italic capitalize" />]}
            />
          </p>
        </div>
        <div className="w-full flex flex-col gap-1 justify-start">
          <div
            className="w-full flex items-center cursor-pointer"
            onClick={onClickWholeShares}
          >
            <input
              id="fractional-shares-checkbox"
              type="checkbox"
              className="w-4 h-4 accent-neutral-500 cursor-pointer"
              checked={!useWholeShares}
              onChange={onClickWholeShares}
            />
            <label
              htmlFor="#fractional-shares-checkbox"
              className="ml-2 cursor-pointer select-none"
            >
              <Trans
                i18nKey="investStep.useFractionalShares"
                values={{
                  fractionalShares: t("investStep.fractionalShares"),
                }}
                components={[<span className="font-medium capitalize" />]}
              />
            </label>
          </div>
          <p className="text-sm font-light">
            <Trans
              i18nKey="investStep.useFractionalSharesInfo"
              values={{
                message: t("investStep.fractionalShares"),
              }}
              components={[<span className="italic capitalize" />]}
            />
          </p>
        </div>
      </div>

      <div className="w-full mt-6 flex items-center justify-between">
        <Button variant="link" size="link" onClick={onClickGoBack}>
          {t("common.goBack")}{" "}
        </Button>
        <Button
          onClick={onClickRunAllocation}
          disabled={isRunAllocationDisabled}
          data-testid="invest.runAllocation.button"
        >
          {t("investStep.runAllocation")}{" "}
        </Button>
      </div>
    </div>
  );
};
