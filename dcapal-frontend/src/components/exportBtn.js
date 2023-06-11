import React from "react";
import { useSelector } from "react-redux";
import { Step } from "../app/appSlice";
import { aclassToString } from "./allocationFlow/portfolioStep/portfolioSlice";

const pad = (n) => `${n}`.padStart(2, "0");

const exportPfolio = (pfolio) => {
  const assets = Object.values(pfolio.assets).map((a) => ({
    symbol: a.symbol,
    name: a.name,
    aclass: aclassToString(a.aclass),
    baseCcy: a.baseCcy,
    provider: a.provider,
    price: a.price,
    qty: a.qty,
    amount: a.amount,
    weight: a.weight,
    targetWeight: a.targetWeight,
  }));

  const data = { quoteCcy: pfolio.quoteCcy, assets: assets };
  const jsonString = `data:text/json;chatset=utf-8,${encodeURIComponent(
    JSON.stringify(data, null, 2)
  )}`;
  const link = document.createElement("a");
  link.href = jsonString;

  const now = new Date();
  const [year, month, day] = [now.getFullYear(), now.getMonth(), now.getDay()];
  const [hh, mm, ss] = [now.getHours(), now.getMinutes(), now.getSeconds()];
  const ts = `${pad(year)}${pad(month)}${pad(day)}${pad(hh)}${pad(mm)}${pad(
    ss
  )}`;
  link.download = `dcapal-portfolio-${ts}.json`;

  link.click();
};

export const ExportBtn = () => {
  const step = useSelector((state) => state.app.allocationFlowStep);

  const pfolio = useSelector((state) => ({
    assets: state.pfolio.assets,
    quoteCcy: state.pfolio.quoteCcy,
  }));

  const isDisplay =
    step && step === Step.PORTFOLIO && Object.keys(pfolio.assets).length > 0;

  const onClick = () => {
    exportPfolio(pfolio);
  };

  return (
    <button
      style={{ display: isDisplay ? "flex" : "none" }}
      className="px-3 py-2 flex justify-center items-center whitespace-nowrap bg-neutral-500 hover:bg-neutral-600 active:bg-neutral-800 text-white rounded-md shadow-md disabled:pointer-events-none disabled:opacity-60"
      onClick={onClick}
    >
      Export Portfolio
    </button>
  );
};
