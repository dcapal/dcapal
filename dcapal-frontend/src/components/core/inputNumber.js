import React, { useState } from "react";

export const InputNumberType = Object.freeze({
  INTEGRAL: 0,
  DECIMAL: 1,
});

export const InputNumber = ({
  type,
  value,
  onChange,
  isValid,
  textAlign,
  min,
  max,
  height,
  textSize,
}) => {
  const handleChange = (e) => {
    if (e.target.value < 0) {
      e.preventDefault();
      return;
    }

    let newValue = e.target.value;
    if (type === InputNumberType.INTEGRAL && newValue.startsWith("0")) {
      newValue = newValue.substr(1);
    }

    if (newValue === "") {
      newValue = 0;
    } else if (type === InputNumberType.DECIMAL) {
      newValue = parseFloat(newValue);
    } else {
      newValue = parseInt(newValue);
    }

    onChange(newValue);
  };

  const invalidClass = isValid
    ? ""
    : "border-red-300 hover:border-red-500 focus-visible:outline-red-600";
  const className = `w-full px-2 pt-1 pb-1.5 border focus-visible:outline-1 rounded-md border-gray-300 hover:border-gray-500 focus-visible:outline-gray-600 ${textAlign} ${invalidClass}`;
  const placeholder = type === InputNumberType.INTEGRAL ? "0" : "0.0";

  return (
    <input
      style={{
        height: height ? `${height}px` : "unset",
        fontSize: textSize ? `${textSize}px` : "unset",
      }}
      className={className}
      placeholder={placeholder}
      type={"number"}
      value={value !== 0 ? value : ""}
      onChange={handleChange}
      min={min}
      max={max}
    />
  );
};
