import React, { useState } from "react";
import { useSelector } from "react-redux";
import { Step } from "../../app/appSlice";
import { CcyStep } from "./ccyStep";
import { EndStep } from "./endStep";
import { ImportStep } from "./importStep";
import { InitStep } from "./initStep";
import { InvestStep } from "./investStep";
import { PortfolioStep } from "./portfolioStep";

export const AllocationFlow = () => {
  const step = useSelector((state) => state.app.allocationFlowStep);

  if (step <= Step.CCY) {
    return <CcyStep />;
  } else if (step === Step.IMPORT) {
    return <ImportStep />;
  } else if (step === Step.PORTFOLIO) {
    return <PortfolioStep />;
  } else if (step === Step.INVEST) {
    return <InvestStep />;
  } else if (step === Step.END) {
    return <EndStep />;
  }
};
