import React, { useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import { useMediaQuery } from "@react-hook/media-query";

import { InputNumber, InputNumberType } from "../../core/inputNumber";
import { removeAsset } from "./portfolioSlice";
import { MEDIA_SMALL } from "../../../app/config";

export const AssetCard = ({
  symbol,
  name,
  price,
  qty,
  weight,
  targetWeight,
  isValidTargetWeight,
  ...props
}) => {
  const quoteCcy = useSelector((state) => state.pfolio.quoteCcy);
  const dispatch = useDispatch();
  const isMobile = !useMediaQuery(MEDIA_SMALL);

  const priceFmt = {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  };

  const onClickDelete = () => {
    dispatch(removeAsset({ symbol: symbol }));
  };

  return (
    <div className="relative w-full max-w-[36rem] flex flex-col my-2 first:mt-0 px-3 pt-2 pb-3 shadow-md ring-1 ring-black/5 rounded-md bg-white">
      <div
        className="absolute flex justify-center items-center w-8 h-8 -right-4 -top-4 rounded-full bg-slate-500 cursor-pointer"
        onClick={onClickDelete}
      >
        <div className="text-white mb-1">x</div>
      </div>
      <div className="mb-2 flex items-center justify-between">
        <div className="min-w-0 flex flex-col">
          <div className="text-lg truncate font-medium capitalize" title={name}>
            {name}
          </div>
          <div className="text-sm font-light uppercase">{symbol}</div>
        </div>
        <div className="grow flex items-center justify-end">
          {!isMobile && (
            <div className="flex ml-4">
              <div className="font-medium">
                {(price * qty).toLocaleString("en-US", priceFmt)}
              </div>
              <div className="ml-1 uppercase">{quoteCcy}</div>
            </div>
          )}
          <div className="whitespace-nowrap ml-4 py-1 px-2 bg-indigo-400/50 text-indigo-800 font-semibold rounded-md">
            {weight.toLocaleString("en-US", {
              minimumFractionDigits: 1,
              maximumFractionDigits: 1,
            })}{" "}
            %
          </div>
        </div>
      </div>
      {isMobile && (
        <div className="flex flex-col">
          <div className="flex items-center py-2">
            <div className="min-w-[6rem] mr-2 font-light text-xs">Amount</div>
            <div className="uppercase text-sm">{quoteCcy}</div>
            <div className="ml-1 text-sm">
              {(price * qty).toLocaleString("en-US", priceFmt)}
            </div>
          </div>
          <div className="flex items-center py-2">
            <div className="min-w-[6rem] mr-2 font-light text-xs">Price</div>
            <div className="uppercase text-sm">{quoteCcy}</div>
            <div className="ml-1 text-sm">
              {price.toLocaleString("en-US", priceFmt)}
            </div>
          </div>
          <div className="flex items-center h-12">
            <div className="min-w-[6rem] mr-2 font-light text-xs">Quantity</div>
            <div className="grow">
              <InputNumber
                textAlign={"text-right"}
                type={InputNumberType.INTEGRAL}
                value={qty}
                onChange={props.setQty}
                isValid={true}
                min={0}
              />
            </div>
          </div>
          <div className="flex items-center h-12">
            <div className="min-w-[6rem] font-light text-xs">
              Target weight (%)
            </div>
            <div className="grow ml-2">
              <InputNumber
                textAlign={"text-right"}
                type={InputNumberType.DECIMAL}
                value={targetWeight}
                onChange={props.setTargetWeight}
                isValid={isValidTargetWeight}
                min={0}
                max={100}
              />
            </div>
          </div>
        </div>
      )}
      {!isMobile && (
        <div className="flex justify-between items-start">
          <div className="flex flex-col">
            <div className="flex items-center h-12">
              <div className="w-12 mr-2 font-light text-xs">Quantity</div>
              <div className="w-40">
                <InputNumber
                  textAlign={"text-left"}
                  type={InputNumberType.INTEGRAL}
                  value={qty}
                  onChange={props.setQty}
                  isValid={true}
                  min={0}
                />
              </div>
            </div>
            <div className="flex items-center h-6">
              <div className="w-12 mr-2 font-light text-xs">Price</div>
              <div className="uppercase text-sm">{quoteCcy}</div>
              <div className="ml-1 text-sm">
                {price.toLocaleString("en-US", priceFmt)}
              </div>
            </div>
          </div>
          <div className="flex items-center mr-2 h-12">
            <div className="font-light text-xs">Target weight</div>
            <div className="w-28 ml-2">
              <InputNumber
                textAlign={"text-right"}
                type={InputNumberType.DECIMAL}
                value={targetWeight}
                onChange={props.setTargetWeight}
                isValid={isValidTargetWeight}
                min={0}
                max={100}
              />
            </div>
            <div className="ml-1">%</div>
          </div>
        </div>
      )}
    </div>
  );
};
