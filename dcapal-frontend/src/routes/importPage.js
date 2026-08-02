import React, { useEffect } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { Step, useAppStore } from "@/state/appStore";
import { Spinner } from "@components/spinner/spinner";
import { useFetchImportedPortfolio } from "@hooks/useFetchImportedPortfolio";

import IMPORT_PORTFOLIO_SVG from "@images/headers/import-portfolio.svg";

const navigateToPortfolios = (
  portfolio,
  step,
  setPfolioFile,
  setAllocationFlowStep,
  navigate
) => {
  setPfolioFile({ file: portfolio ? JSON.stringify(portfolio) : "" });
  setAllocationFlowStep({ step });
  navigate("/allocate");
};

/** Loads a shared portfolio link and routes it into the allocation flow. */
export default function ImportPage() {
  const navigate = useNavigate();
  const setAllocationFlowStep = useAppStore(
    (state) => state.setAllocationFlowStep
  );
  const setPfolioFile = useAppStore((state) => state.setPfolioFile);
  const location = useLocation();
  const { t } = useTranslation();

  const searchParams = new URLSearchParams(location.search);
  const portfolioId = searchParams.get("p");

  const { portfolio, isLoading } = useFetchImportedPortfolio(portfolioId);

  useEffect(() => {
    if (isLoading) return;

    if (!portfolio) {
      navigateToPortfolios(
        null,
        Step.PORTFOLIOS,
        setPfolioFile,
        setAllocationFlowStep,
        navigate
      );
    } else {
      navigateToPortfolios(
        portfolio,
        Step.IMPORT,
        setPfolioFile,
        setAllocationFlowStep,
        navigate
      );
    }
  }, [isLoading, navigate, portfolio, setAllocationFlowStep, setPfolioFile]);

  return (
    <div
      data-testid="route-import"
      className="w-full flex flex-col items-center"
    >
      <div className="w-full px-6 py-10 flex flex-col grow justify-center items-center text-center gap-8">
        <img
          className="w-full px-4 sm:max-w-[20rem] pb-2"
          alt="Import Portfolio"
          src={IMPORT_PORTFOLIO_SVG}
        />
        <h1 className="text-3xl font-bold">
          {t("importStep.importPortfolio")}
        </h1>
        <span className="flex flex-col gap-y-2 items-center font-light">
          <p>{t("importStep.fetchData")}...</p>
        </span>
        <Spinner />
      </div>
    </div>
  );
}
