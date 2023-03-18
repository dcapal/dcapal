import React, { useEffect, useState } from "react";

export const InputNumberType = Object.freeze({
  INTEGRAL: 0,
  DECIMAL: 1,
});

const handleFocus = (event) => {
  setTimeout(() => event.target.select(), 0);
};

export const InputNumber = ({
  type,
  value,
  onChange,
  isValid,
  textAlign,
  min,
  max,
  step,
  textSize,
}) => {
  const [state, setState] = useState(value || value === 0 ? value : "");

  useEffect(() => {
    setState(value || value === 0 ? value : "");
  }, [value]);

  const handleChange = (e) => {
    setState(e.target.value);
  };

  const handleOnBlur = (e) => {
    e.preventDefault();
    if (e.target.value < 0) {
      e.preventDefault();
      return;
    }

    let newValue = e.target.value;
    if (type === InputNumberType.INTEGRAL && newValue.startsWith("0")) {
      newValue = newValue.substr(1);
    }

    if (newValue === "") {
      newValue = null;
    } else if (type === InputNumberType.DECIMAL) {
      newValue = parseFloat(newValue);
    } else {
      newValue = parseInt(newValue);
    }

    if (newValue === NaN) {
      newValue = null;
    }

    onChange(newValue);
  };

  const invalidClass = isValid
    ? ""
    : "border-red-300 hover:border-red-500 focus-visible:outline-red-600";
  const className = `w-full px-2 pt-1 pb-1.5 leading-none border focus-visible:outline-1 rounded-md border-gray-300 hover:border-gray-500 focus-visible:outline-gray-600 ${textAlign} ${invalidClass}`;
  const placeholder = type === InputNumberType.INTEGRAL ? "0" : "0.0";

  return (
    <input
      style={{
        fontSize: textSize ? `${textSize}` : "unset",
      }}
      className={className}
      placeholder={placeholder}
      type={"number"}
      value={state}
      onChange={handleChange}
      onBlur={handleOnBlur}
      onFocus={handleFocus}
      min={min}
      max={max}
      step={step ? step : "any"}
    />
  );
};
