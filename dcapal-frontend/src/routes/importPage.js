import React, { useEffect } from "react";
import { useDispatch } from "react-redux";
import { useLocation, useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import axios from "axios";

import { setAllocationFlowStep, setPfolioFile, Step } from "../app/appSlice";
import { DCAPAL_API } from "../app/config";
import { api } from "../app/api";

import IMPORT_PORTFOLIO from "@images/headers/import-portfolio.svg";
import { Spinner } from "../components/spinner/spinner";

const fetchImportedPortfolio = async (id) => {
  const url = `${DCAPAL_API}/import/portfolio/${id}`;
  try {
    const response = await api.get(url);

    if (response.status != 200) {
      console.error(
        `Failed to fetch imported portfolio (${id}): {status: ${response.status}, data: ${response.data}}`
      );
      return null;
    }

    return response.data;
  } catch (error) {
    console.error(error);
    return null;
  }
};

const navigateToPortfolios = (dispatch, navigate) => {
  dispatch(setPfolioFile({ file: "" }));
  dispatch(setAllocationFlowStep({ step: Step.PORTFOLIOS }));
  navigate("/allocate");
};

export default function ImportPage() {
  const navigate = useNavigate();
  const dispatch = useDispatch();
  const location = useLocation();
  const { t } = useTranslation();

  const searchParams = new URLSearchParams(location.search);
  const portfolioId = searchParams.get("p");

  useEffect(() => {
    if (!portfolioId) return navigateToPortfolios(dispatch, navigate);

    const fetchPortfolio = async () => {
      const p = await fetchImportedPortfolio(portfolioId);
      if (!p) return navigateToPortfolios(dispatch, navigate);

      dispatch(setPfolioFile({ file: JSON.stringify(p) }));
      dispatch(setAllocationFlowStep({ step: Step.IMPORT }));
      navigate("/allocate");
    };

    fetchPortfolio();
  }, [portfolioId]);

  return (
    <div className="w-full flex flex-col items-center">
      <div className="w-full px-6 py-10 flex flex-col grow justify-center items-center text-center gap-8">
        <img
          className="w-full px-4 sm:max-w-[20rem] pb-2"
          alt="Import Portfolio"
          src={IMPORT_PORTFOLIO}
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
