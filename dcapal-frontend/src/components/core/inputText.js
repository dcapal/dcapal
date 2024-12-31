import classNames from "classnames";
import React, { useEffect, useState } from "react";
import { Input } from "@/components/ui/input";

const handleFocus = (event) => {
  setTimeout(() => event.target.select(), 0);
};

export const InputText = ({
  value,
  onChange,
  isValid,
  textSize,
  leadingNone,
}) => {
  const [state, setState] = useState(value || "");

  useEffect(() => {
    setState(value || "");
  }, [value]);

  const handleChange = (e) => {
    setState(e.target.value);
  };

  const handleOnBlur = (e) => {
    let newValue = e.target.value.trim();
    onChange(newValue);
  };

  return (
    <Input
      style={{
        fontSize: textSize ? `${textSize}` : "unset",
      }}
      className={classNames({
        "border-destructive focus-visible:outline-destructive": !isValid,
        "leading-none": leadingNone,
        "leading-normal": !leadingNone,
      })}
      type={"text"}
      value={state}
      onChange={handleChange}
      onBlur={handleOnBlur}
      onFocus={handleFocus}
    />
  );
};
