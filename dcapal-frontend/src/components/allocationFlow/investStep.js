import React, { useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import { Link } from "react-router-dom";
import { setAllocationFlowStep, Step } from "../../app/appSlice";
import { InputNumber, InputNumberType } from "../core/inputNumber";
import { setBudget } from "./portfolioStep/portfolioSlice";

export const InvestStep = ({ useTaxEfficient, setUseTaxEfficient }) => {
  const [cash, setCash] = useState(0);
  const dispatch = useDispatch();

  const quoteCcy = useSelector((state) => state.pfolio.quoteCcy);

  const onChangeTaxEfficient = (e) => {
    setUseTaxEfficient(e.target.checked || false);
  };

  const onClickGoBack = () => {
    dispatch(setAllocationFlowStep({ step: Step.PORTFOLIO }));
  };

  const onClickRunAllocation = () => {
    dispatch(setBudget({ budget: cash }));
    dispatch(setAllocationFlowStep({ step: Step.END }));
  };

  return (
    <div className="w-full h-full flex flex-col items-center">
      <div className="mt-2 mb-8 text-3xl font-light">
        How much you would like to allocate?
      </div>
      <div className="w-full flex justify-center items-end">
        <div className="w-full">
          <InputNumber
            height="68"
            textSize="60"
            textAlign="text-right"
            type={InputNumberType.DECIMAL}
            value={cash}
            onChange={setCash}
            isValid={true}
            min={0}
          />
        </div>
        <div className="ml-2 pb-2 text-2xl font-light uppercase">
          {quoteCcy}
        </div>
      </div>
      <div className="w-full flex flex-col gap-1 justify-start">
        <div className="w-full mt-20 flex">
          <div>
            <input
              id="tax-efficient-checkbox"
              type="checkbox"
              className="w-4 h-4 accent-neutral-500"
              checked={useTaxEfficient}
              onChange={onChangeTaxEfficient}
            />
            <label htmlFor="#tax-efficient-checkbox" className="ml-2">
              Use <span className="font-medium">Tax Efficient</span> algorithm
            </label>
          </div>
        </div>
        <p className="text-sm font-light">
          With <span className="italic">Tax Efficient</span> option, we do our
          best to rebalance your portfolio using your liquidity, with buy-only
          suggestions. Otherwise, we might suggest to sell part of your
          positions.
        </p>
      </div>
      <div className="w-full mt-6 flex items-center justify-between">
        <span
          className="font-medium underline cursor-pointer"
          onClick={onClickGoBack}
        >
          Go back
        </span>
        <div
          className="px-3 py-2 flex justify-center items-center cursor-pointer bg-neutral-500 hover:bg-neutral-600 active:bg-neutral-800 text-white text-lg rounded-md shadow-md"
          onClick={onClickRunAllocation}
        >
          Run Allocation
        </div>
      </div>
    </div>
  );
};
