import React from "react";
import { useDispatch, useSelector } from "react-redux";
import {
  FeeType,
  setFeeType,
  setFeeTypeAsset,
  setFixedFeeAmount,
  setFixedFeeAmountAsset,
  setMaxFeeImpact,
  setMaxFeeImpactAsset,
  setVariableFee,
  setVariableFeeAsset,
} from "./portfolioSlice";
import classNames from "classnames";
import { InputNumber, InputNumberType } from "../../core/inputNumber";

export const TransactionFees = ({ asset }) => {
  const dispatch = useDispatch();
  const quoteCcy = useSelector((state) => state.pfolio.quoteCcy);

  const fees = useSelector((state) => {
    return asset && asset in state.pfolio.assets
      ? state.pfolio.assets[asset].fees || state.pfolio.fees
      : state.pfolio.fees;
  });

  const feeType = useSelector((state) => {
    return asset
      ? asset in state.pfolio.assets && state.pfolio.assets[asset].fees
        ? state.pfolio.assets[asset].fees.feeStructure.type
        : null
      : state.pfolio.fees.feeStructure.type;
  });

  const maxFeeImpact = fees?.maxFeeImpact || null;
  const fixedFeeAmount = fees?.feeStructure?.feeAmount || null;
  const variableFee =
    feeType === FeeType.VARIABLE
      ? {
          feeRate: fees?.feeStructure?.feeRate || null,
          minFee: fees?.feeStructure?.minFee || null,
          maxFee: fees?.feeStructure?.maxFee || null,
        }
      : null;

  const setSelected = (type) => {
    if (!asset) {
      dispatch(setFeeType({ type: type }));
    } else {
      dispatch(setFeeTypeAsset({ type: type, symbol: asset }));
    }
  };

  const onChangeMaxFeeImpact = (value) => {
    if (!asset) {
      dispatch(setMaxFeeImpact({ value: value }));
    } else {
      dispatch(setMaxFeeImpactAsset({ value: value, symbol: asset }));
    }
  };

  const onChangeFeeAmount = (value) => {
    if (!asset) {
      dispatch(setFixedFeeAmount({ value: value }));
    } else {
      dispatch(setFixedFeeAmountAsset({ value: value, symbol: asset }));
    }
  };

  const onChangeVariableFee = (value) => {
    if (!asset) {
      dispatch(setVariableFee({ ...value }));
    } else {
      dispatch(setVariableFeeAsset({ ...value, symbol: asset }));
    }
  };

  return (
    <div className="w-full flex flex-col gap-2 items-start justify-center">
      <FeeGroup selected={feeType} setSelected={setSelected} asset={asset} />
      {feeType === FeeType.ZERO_FEE && <NoFeesForm />}
      {feeType === FeeType.FIXED && (
        <FixedFeeForm
          quoteCcy={quoteCcy}
          maxFeeImpact={maxFeeImpact}
          onChangeMaxFeeImpact={onChangeMaxFeeImpact}
          feeAmount={fixedFeeAmount}
          onChangeFeeAmount={onChangeFeeAmount}
        />
      )}
      {feeType === FeeType.VARIABLE && (
        <VariableFeeForm
          quoteCcy={quoteCcy}
          maxFeeImpact={maxFeeImpact}
          onChangeMaxFeeImpact={onChangeMaxFeeImpact}
          variableFee={variableFee}
          onChangeVariableFee={onChangeVariableFee}
        />
      )}
    </div>
  );
};

const FeeGroup = ({ selected, setSelected, asset }) => (
  <div className="flex w-full">
    {asset && (
      <FeeRadio
        key="default"
        type={null}
        label="Default"
        selected={selected}
        setSelected={setSelected}
      />
    )}
    <FeeRadio
      key="zeroFees"
      type={FeeType.ZERO_FEE}
      label="No fees"
      selected={selected}
      setSelected={setSelected}
    />
    <FeeRadio
      key="fixed"
      type={FeeType.FIXED}
      label="Fixed"
      selected={selected}
      setSelected={setSelected}
    />
    <FeeRadio
      key="variable"
      type={FeeType.VARIABLE}
      label="Variable"
      selected={selected}
      setSelected={setSelected}
    />
  </div>
);

const FeeRadio = ({ type, label, selected, setSelected }) => {
  const isSelected = selected === type;

  const onClick = () => {
    setSelected(type);
  };

  const commonClass =
    "px-2 py-1 flex grow justify-center items-center border hover:bg-neutral-600 hover:border-neutral-600 hover:text-white active:bg-neutral-800 active:text-white first:rounded-l-md last:rounded-r-md cursor-pointer select-none";

  const className = classNames(commonClass, {
    "border-gray-300": !isSelected,
    "bg-white": !isSelected,
    "border-gray-500": isSelected,
    "bg-neutral-500": isSelected,
    "text-white": isSelected,
  });

  return (
    <div className={className} onClick={onClick}>
      {label}
    </div>
  );
};

const NoFeesForm = () => {
  return (
    <p className="font-light text-center self-center">
      💰{" "}
      <span className="italic">
        Enjoy your zero-fee trading life, lucky bastard
      </span>
    </p>
  );
};

const FixedFeeForm = ({
  quoteCcy,
  maxFeeImpact,
  onChangeMaxFeeImpact,
  feeAmount,
  onChangeFeeAmount,
}) => {
  return (
    <div className="w-full flex flex-col justify-center gap-1">
      <MaxFeeImpactInput
        quoteCcy={quoteCcy}
        maxFeeImpact={maxFeeImpact}
        onChangeMaxFeeImpact={onChangeMaxFeeImpact}
      />
      <div className="w-full flex items-center">
        <label className="min-w-[8rem] mr-2 font-light">Fee amount</label>
        <div className="grow">
          <InputNumber
            textAlign="text-right"
            type={InputNumberType.DECIMAL}
            value={feeAmount}
            onChange={onChangeFeeAmount}
            isValid={true}
            min={0}
          />
        </div>
        <label className="text-start min-w-[2rem] ml-2 uppercase">
          {quoteCcy}
        </label>
      </div>
    </div>
  );
};

const VariableFeeForm = ({
  quoteCcy,
  maxFeeImpact,
  onChangeMaxFeeImpact,
  variableFee,
  onChangeVariableFee,
}) => {
  const { feeRate, minFee, maxFee } = variableFee;

  const onChangeFeeRate = (value) => {
    onChangeVariableFee({
      feeRate: value,
    });
  };

  const onChangeMinFee = (value) => {
    onChangeVariableFee({
      minFee: value,
    });
  };

  const onChangeMaxFee = (value) => {
    onChangeVariableFee({
      maxFee: value,
    });
  };

  const isMinFeeValid = (() => {
    if (minFee && maxFee) {
      return minFee <= maxFee;
    }

    return true;
  })();

  return (
    <div className="w-full flex flex-col justify-center gap-1">
      <MaxFeeImpactInput
        quoteCcy={quoteCcy}
        maxFeeImpact={maxFeeImpact}
        onChangeMaxFeeImpact={onChangeMaxFeeImpact}
      />
      <div className="w-full flex items-center">
        <label className="min-w-[8rem] mr-2 font-light">Fee percentage</label>
        <div className="grow">
          <InputNumber
            textAlign="text-right"
            type={InputNumberType.DECIMAL}
            value={feeRate}
            onChange={onChangeFeeRate}
            isValid={true}
            min={0}
            max={100}
          />
        </div>
        <label className="text-start min-w-[2rem] ml-2 uppercase">%</label>
      </div>
      <div className="w-full flex items-center">
        <label className="min-w-[8rem] mr-2 font-light">Min fee</label>
        <div className="grow">
          <InputNumber
            textAlign="text-right"
            type={InputNumberType.DECIMAL}
            value={minFee}
            onChange={onChangeMinFee}
            isValid={isMinFeeValid}
            min={0}
          />
        </div>
        <label className="text-start min-w-[2rem] ml-2 uppercase">
          {quoteCcy}
        </label>
      </div>
      <div className="w-full flex items-center">
        <label className="min-w-[8rem] mr-2 font-light">Max fee</label>
        <div className="grow">
          <InputNumber
            textAlign="text-right"
            type={InputNumberType.DECIMAL}
            value={maxFee}
            onChange={onChangeMaxFee}
            isValid={true}
            min={0}
          />
        </div>
        <label className="text-start min-w-[2rem] ml-2 uppercase">
          {quoteCcy}
        </label>
      </div>
      {!isMinFeeValid && (
        <div className="mt-2 font-light text-red-500 text-center">
          Review your <span className="font-normal">Min fee</span>. Must be less
          than or equal to max fee.
        </div>
      )}
    </div>
  );
};

const MaxFeeImpactInput = ({ maxFeeImpact, onChangeMaxFeeImpact }) => {
  return (
    <div className="w-full flex items-center">
      <label className="min-w-[8rem] mr-2 font-light">Max fee impact</label>
      <div className="grow">
        <InputNumber
          textAlign="text-right"
          type={InputNumberType.DECIMAL}
          value={maxFeeImpact}
          onChange={onChangeMaxFeeImpact}
          isValid={true}
          min={0}
          max={100}
        />
      </div>
      <label className="text-start min-w-[2rem] ml-2 uppercase">%</label>
    </div>
  );
};
