import { useMediaQuery } from "@react-hook/media-query";
import React from "react";
import { useSelector } from "react-redux";
import { MEDIA_SMALL } from "../../../app/config";
import { UNALLOCATED_CASH } from ".";
import { roundAmount, roundDecimals } from "../../../utils";
import { ACLASS, FeeType } from "../portfolioStep/portfolioSlice";
import { useTranslation } from "react-i18next";

const feeAmount = (fees, amount) => {
  if (!fees) return 0;

  switch (fees.feeStructure.type) {
    case FeeType.ZERO_FEE:
      return 0;
    case FeeType.FIXED:
      return amount > 0 ? fees.feeStructure.feeAmount : 0;
    case FeeType.VARIABLE:
      return Math.min(
        Math.max(
          (fees.feeStructure.feeRate * amount) / 100,
          fees.feeStructure.minFee
        ),
        fees.feeStructure.maxFee
      );
    default:
      return 0;
  }
};

export const AllocateCard = ({
  symbol,
  name,
  aclass,
  qty,
  oldQty,
  price,
  amount,
  oldAmount,
  weight,
  oldWeight,
  targetWeight,
  fees,
  theoAlloc,
}) => {
  const { t } = useTranslation();
  const quoteCcy = useSelector((state) => state.pfolio.quoteCcy);
  const isMobile = !useMediaQuery(MEDIA_SMALL);

  const amtFmt = {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  };
  const priceFmt = {
    minimumFractionDigits: 2,
    maximumFractionDigits: 4,
  };
  const weightFmt = {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  };

  const diffAmount = (amount || 0) - (oldAmount || 0);
  const diffSign = diffAmount === 0 ? " " : diffAmount > 0 ? "+" : "-";
  const diffAmountTxt = Math.abs(diffAmount).toLocaleString("en-US", priceFmt);

  const actionIcon = qty === oldQty ? "🏦" : qty > oldQty ? "📈" : "📉";

  const amountFees = feeAmount(fees, Math.abs(diffAmount));
  const feeImpact =
    diffAmount === 0 ? 0 : (amountFees / Math.abs(diffAmount)) * 100;

  const theo = (() => {
    if (!theoAlloc || theoAlloc.shares === oldQty) return null;

    const diffQty = Math.abs(theoAlloc.shares - oldQty);
    const action =
      theoAlloc.shares > oldQty ? "endStep.bought" : "endStep.sold";
    const feeImpact =
      (theoAlloc.fees / Math.abs(theoAlloc.amount - (oldAmount || 0))) * 100;

    return {
      diffQty: diffQty,
      action: action,
      fees: theoAlloc.fees,
      amount: theoAlloc.amount,
      feeImpact: feeImpact,
    };
  })();

  const bgColor =
    diffAmount > 0
      ? "bg-green-800"
      : diffAmount < 0
      ? "bg-red-300"
      : "bg-neutral-500";

  const textColor =
    diffAmount > 0
      ? "text-green-50"
      : diffAmount < 0
      ? "text-red-800"
      : "text-white";

  const diffAmountClass = `ml-4 py-1 px-2 whitespace-nowrap ${bgColor} ${textColor} font-semibold rounded-md`;

  if (name === UNALLOCATED_CASH) {
    return (
      <div className="w-full max-w-[36rem] flex flex-col my-1 first:mt-0 px-3 pt-2 pb-3 shadow-md ring-1 ring-black/5 rounded-md bg-white">
        <div className="mb-2 flex justify-between items-center">
          <div className="min-w-0 flex flex-col">
            <div
              className="text-lg truncate font-medium capitalize"
              title={name}
            >
              {name}
            </div>
            <div className="text-sm font-light uppercase">{symbol}</div>
          </div>
          <div className={diffAmountClass}>
            <span className="mr-1">{diffSign}</span>
            {diffAmountTxt}
            <span className="ml-1 uppercase">{quoteCcy}</span>
          </div>
        </div>
        <p className="font-light text-center">
          💰 <span className="italic">{t("endStep.leftBudget")}</span>
        </p>
      </div>
    );
  } else {
    return (
      <div className="w-full max-w-[36rem] flex flex-col my-1 first:mt-0 px-3 pt-2 pb-3 shadow-md ring-1 ring-black/5 rounded-md bg-white">
        <div className="mb-2 flex justify-between items-center">
          <div className="min-w-0 flex flex-col">
            <div
              className="text-lg truncate font-medium capitalize"
              title={name}
            >
              {name}
            </div>
            <div className="text-sm font-light uppercase">{symbol}</div>
          </div>
          <div className={diffAmountClass}>
            <span className="mr-1">{diffSign}</span>
            {diffAmountTxt}
            <span className="ml-1 uppercase">{quoteCcy}</span>
          </div>
        </div>
        {qty >= 0 && (
          <div>
            <div className="flex gap-1 px-3 py-2 mt-[5px] rounded-md bg-blue-100/50">
              <span>{actionIcon}</span>
              {qty > oldQty && (
                <span className="font-light">
                  {t("endStep.buy")} {roundDecimals(qty - oldQty, 6)}{" "}
                  <span className="uppercase">{symbol}</span> @ {price}{" "}
                  <span className="uppercase">{quoteCcy}</span>
                </span>
              )}
              {qty < oldQty && (
                <span className="font-light">
                  {t("endStep.sell")} {roundDecimals(oldQty - qty, 6)}{" "}
                  <span className="uppercase">{symbol}</span> @ {price}{" "}
                  <span className="uppercase">{quoteCcy}</span>
                </span>
              )}
              {qty === oldQty && (
                <span className="font-light italic">
                  {aclass === ACLASS.CRYPTO
                    ? t("endStep.nothingToTradeHodl")
                    : t("endStep.nothingToTradeHold")}
                </span>
              )}
            </div>
            <p className="px-2 my-3 border border-gray-200/50" />
          </div>
        )}
        <div className="flex flex-col gap-1">
          {isMobile && (
            <div className="flex flex-col justify-between items-start">
              <div className="w-full flex items-center justify-between">
                <div className="min-w-[5.5rem] mr-2 font-light text-xs">
                  {t("endStep.currentAmount")}
                </div>
                <p className="uppercase">
                  {oldAmount.toLocaleString("en-US", amtFmt)} {quoteCcy}
                </p>
              </div>
              <div className="w-full flex items-center justify-between">
                <div className="min-w-[5.5rem] mr-2 font-light text-xs">
                  {t("endStep.newAmount")}
                </div>
                <p className="uppercase">
                  {amount.toLocaleString("en-US", amtFmt)} {quoteCcy}
                </p>
              </div>
              <div className="w-full flex items-center justify-between">
                <div className="min-w-[5.5rem] mr-2 font-light text-xs">
                  {t("endStep.currentWeight")}
                </div>
                <p>{oldWeight.toLocaleString("en-US", weightFmt)} %</p>
              </div>
              <div className="w-full flex items-center justify-between">
                <div className="min-w-[5.5rem] mr-2 font-light text-xs">
                  {t("endStep.newWeight")}
                </div>
                <p>{weight.toLocaleString("en-US", weightFmt)} %</p>
              </div>
              <div className="w-full flex items-center justify-between">
                <div className="min-w-[5.5rem] mr-2 font-light text-xs">
                  {t("endStep.targetWeight")}
                </div>
                <p>{targetWeight.toLocaleString("en-US", weightFmt)} %</p>
              </div>
              <div className="w-full flex items-center justify-between">
                <div className="min-w-[5.5rem] mr-2 font-light text-xs">
                  {t("endStep.fees")}
                </div>
                <p className="uppercase">
                  {amountFees.toLocaleString("en-US", priceFmt)} {quoteCcy}
                </p>
              </div>
              <div className="w-full flex items-center justify-between">
                <div className="min-w-[5.5rem] mr-2 font-light text-xs">
                  {t("endStep.feeImpact")}
                </div>
                <p>{feeImpact.toLocaleString("en-US", weightFmt)} %</p>
              </div>
            </div>
          )}
          {!isMobile && (
            <div className="flex justify-between items-start">
              <div className="flex flex-col">
                <div className="flex items-center">
                  <div className="min-w-[5.5rem] mr-2 font-light text-xs">
                    {t("endStep.currentWeight")}
                  </div>
                  <span>{oldWeight.toLocaleString("en-US", weightFmt)} %</span>
                </div>
                <div className="flex items-center">
                  <div className="min-w-[5.5rem] mr-2 font-light text-xs">
                    {t("endStep.currentAmount")}
                  </div>
                  <span className="uppercase">
                    {oldAmount.toLocaleString("en-US", amtFmt)} {quoteCcy}
                  </span>
                </div>
                <div className="flex items-center">
                  <div className="min-w-[5.5rem] mr-2 font-light text-xs">
                    {t("endStep.targetWeight")}
                  </div>
                  <div>{targetWeight.toLocaleString("en-US", weightFmt)} %</div>
                </div>
                <div className="flex items-center">
                  <div className="min-w-[5.5rem] mr-2 font-light text-xs">
                    {t("endStep.fees")}
                  </div>
                  <span className="uppercase">
                    <div>
                      {amountFees.toLocaleString("en-US", priceFmt)} {quoteCcy}
                    </div>
                  </span>
                </div>
              </div>
              <div className="flex flex-col">
                <div className="flex items-center mr-2">
                  <div className="min-w-[5.5rem] mr-2 font-light text-xs">
                    {t("endStep.newWeight")}
                  </div>
                  <span className="grow text-right">
                    {weight.toLocaleString("en-US", weightFmt)} %
                  </span>
                </div>
                <div className="flex items-center mr-2">
                  <div className="min-w-[5.5rem] mr-2 font-light text-xs">
                    {t("endStep.newAmount")}
                  </div>
                  <span className="grow uppercase text-right">
                    {amount.toLocaleString("en-US", amtFmt)} {quoteCcy}
                  </span>
                </div>
                <div className="flex items-center mr-2">
                  <div className="min-w-[5.5rem] mr-2 font-light text-xs text-white/0">
                    {t("endStep.spacer")}
                  </div>
                  <span className="grow uppercase text-right text-white/0">
                    {t("endStep.spacer")}
                  </span>
                </div>
                <div className="flex items-center mr-2">
                  <div className="min-w-[5.5rem] mr-2 font-light text-xs">
                    {t("endStep.feeImpact")}
                  </div>
                  <span className="grow text-right">
                    {feeImpact.toLocaleString("en-US", weightFmt)} %
                  </span>
                </div>
              </div>
            </div>
          )}
          {theo && (
            <div className="flex gap-1 px-3 py-2 mt-[5px] rounded-md bg-gray-200/50">
              <span>ℹ️</span>
              <span className="font-light italic">
                {t("endStep.shouldHave")} {t(theo?.action)}{" "}
                {roundDecimals(theo?.diffQty, 6)}{" "}
                <span className="uppercase">{symbol}</span> @{" "}
                {price.toLocaleString("en-US", priceFmt)}{" "}
                <span className="uppercase">{quoteCcy}</span> (
                {roundAmount(theo?.amount).toLocaleString("en-US", amtFmt)}{" "}
                <span className="uppercase">{quoteCcy}</span>){" "}
                {t("endStep.butWouldHavePaid")}{" "}
                {theo?.fees.toLocaleString("en-US", priceFmt)}{" "}
                <span className="uppercase">{quoteCcy}</span>{" "}
                {t("endStep.worthOfFees")} (
                {theo?.feeImpact.toLocaleString("en-US", weightFmt)}%{" "}
                {t("endStep.impact")})
              </span>
            </div>
          )}
        </div>
      </div>
    );
  }
};
