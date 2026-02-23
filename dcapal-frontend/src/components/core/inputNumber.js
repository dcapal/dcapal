import classNames from "classnames";
import React, { useEffect, useState } from "react";
import { Input } from "@/components/ui/input";

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
  leadingNone,
  dataTestId,
}) => {
  const [state, setState] = useState(value || value === 0 ? value : "");

  useEffect(() => {
    setState(value || value === 0 ? value : "");
  }, [value]);

  const handleChange = (e) => {
    setState(e.target.value);
  };

  const handleOnBlur = (e) => {
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

    if (newValue === NaN || newValue < min || newValue > max) {
      newValue = null;
    }

    onChange(newValue);
  };

  const placeholder = type === InputNumberType.INTEGRAL ? "0" : "0.0";

  return (
    <Input
      style={{
        fontSize: textSize ? `${textSize}` : "unset",
      }}
      className={classNames(`${textAlign}`, {
        "border-destructive focus-visible:outline-destructive": !isValid,
        "leading-none": leadingNone,
        "leading-normal": !leadingNone,
      })}
      placeholder={placeholder}
      type={"number"}
      value={state}
      onChange={handleChange}
      onBlur={handleOnBlur}
      onFocus={handleFocus}
      min={min}
      max={max}
      step={step ? step : "any"}
      data-testid={dataTestId}
    />
  );
};
