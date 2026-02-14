import React, { useEffect, useState } from "react";
import { useDispatch } from "react-redux";
import { Step, setAllocationFlowStep } from "@app/appSlice";
import { useAppStore } from "@/state/appStore";
import { usePortfolioStore } from "@/state/portfolioStore";
import { EndStep } from "./steps/end";
import { ImportStep } from "./steps/importFile";
import { InvestStep } from "./steps/invest";
import { PortfolioStep } from "./steps/portfolio";
import { PortfoliosStep } from "./steps/portfolios";

export const AllocationFlow = () => {
  const step = useAppStore((state) => state.allocationFlowStep);
  const selectedPfolio = usePortfolioStore((state) => state.selected);

  const [useTaxEfficient, setUseTaxEfficient] = useState(true);
  const [useWholeShares, setUseWholeShares] = useState(true);
  const [useAllBudget, setUseAllBudget] = useState(true);

  const dispatch = useDispatch();

  useEffect(() => {
    if (step >= Step.PORTFOLIO && !selectedPfolio) {
      dispatch(setAllocationFlowStep({ step: Step.PORTFOLIOS }));
    }
    window.scrollTo(0, 0); // Reset scroll position on step change
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
        useAllBudget={useAllBudget}
        setUseTaxEfficient={setUseTaxEfficient}
        setUseWholeShares={setUseWholeShares}
        setUseAllBudget={setUseAllBudget}
      />
    );
  } else if (step === Step.END) {
    return (
      <EndStep
        useTaxEfficient={useTaxEfficient}
        useAllBudget={useAllBudget}
        useWholeShares={useWholeShares}
      />
    );
  }
};
