import React, { useState } from "react";
import { useSelector } from "react-redux";
import { CcyStep } from "./ccyStep";
import { EndStep } from "./endStep";
import { ImportStep } from "./importStep";
import { InitStep } from "./initStep";
import { InvestStep } from "./investStep";
import { PortfolioStep } from "./portfolioStep";

export const Step = Object.freeze({
  INIT: 0,
  CCY: 10,
  IMPORT: 20,
  PORTFOLIO: 30,
  INVEST: 40,
  END: 50,
});

export const AllocationFlow = ({ ...props }) => {
  const [pfolioFile, setPfolioFile] = useState("");

  const step = useSelector((state) => state.app.allocationFlowStep);

  if (step === Step.INIT) {
    return <InitStep setPfolioFile={setPfolioFile} />;
  } else if (step === Step.CCY) {
    return <CcyStep />;
  } else if (step === Step.IMPORT) {
    return <ImportStep pfolioFile={pfolioFile} setPfolioFile={setPfolioFile} />;
  } else if (step === Step.PORTFOLIO) {
    return <PortfolioStep />;
  } else if (step === Step.INVEST) {
    return <InvestStep />;
  } else if (step === Step.END) {
    return <EndStep />;
  }
};
