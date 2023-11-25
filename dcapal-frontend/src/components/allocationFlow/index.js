import React, { useEffect, useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import { Step, setAllocationFlowStep } from "@app/appSlice";
import { EndStep } from "./steps/end";
import { ImportStep } from "./steps/importFile";
import { InvestStep } from "./steps/invest";
import { PortfolioStep } from "./steps/portfolio";
import { PortfoliosStep } from "./steps/portfolios";

export const AllocationFlow = () => {
  const step = useSelector((state) => state.app.allocationFlowStep);
  const selectedPfolio = useSelector((state) => state.pfolio.selected);

  const [useTaxEfficient, setUseTaxEfficient] = useState(true);
  const [useWholeShares, setUseWholeShares] = useState(true);

  const dispatch = useDispatch();

  useEffect(() => {
    if (step >= Step.PORTFOLIO && !selectedPfolio) {
      dispatch(setAllocationFlowStep({ step: Step.PORTFOLIOS }));
    }
  }, [step]);

  if (step <= Step.PORTFOLIOS) {
    return <PortfoliosStep />;
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
