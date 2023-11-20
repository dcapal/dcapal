import React, { useState } from "react";
import { useSelector } from "react-redux";
import { Step } from "../../app/appSlice";
import { CcyStep } from "./steps/ccy";
import { EndStep } from "./steps/end";
import { ImportStep } from "./steps/importFile";
import { InvestStep } from "./steps/invest";
import { PortfolioStep } from "./steps/portfolio";

export const AllocationFlow = () => {
  const step = useSelector((state) => state.app.allocationFlowStep);

  const [useTaxEfficient, setUseTaxEfficient] = useState(true);
  const [useWholeShares, setUseWholeShares] = useState(true);

  if (step <= Step.CCY) {
    return <CcyStep />;
  } else if (step === Step.IMPORT) {
    return <ImportStep />;
  } else if (step === Step.PORTFOLIO) {
    return <PortfolioStep />;
  } else if (step === Step.INVEST) {
    return (
      <InvestStep
        useTaxEfficient={useTaxEfficient}
        useWholeShares={useWholeShares}
        setUseTaxEfficient={setUseTaxEfficient}
        setUseWholeShares={setUseWholeShares}
      />
    );
  } else if (step === Step.END) {
    return (
      <EndStep
        useTaxEfficient={useTaxEfficient}
        useWholeShares={useWholeShares}
      />
    );
  }
};
