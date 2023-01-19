import React, { useEffect } from "react";
import { useDispatch } from "react-redux";
import { useLocation, useSearchParams } from "react-router-dom";
import { useRoutes } from "react-router-dom";
import { App } from "../app";
import { setAllocationFlowStep, setPfolioFile, Step } from "../app/appSlice";
import { clearPortfolio } from "../components/allocationFlow/portfolioStep/portfolioSlice";
import ErrorPage from "./errorPage";
import NotFoundPage from "./notFoundPage";
import { Root } from "./root";
import UnderConstructionPage from "./underConstruction";

export const Router = () => {
  const dispatch = useDispatch();
  let [searchParams] = useSearchParams();
  const location = useLocation();

  useEffect(() => {
    const refId = searchParams.get("refid") || "";

    if (location.pathname !== "/allocate" && refId !== "allocate") {
      dispatch(clearPortfolio({}));
      dispatch(setAllocationFlowStep({ step: Step.CCY }));
      dispatch(setPfolioFile({ file: "" }));
    }
  }, [location]);

  const routes = useRoutes([
    {
      path: "*",
      element: <NotFoundPage />,
    },
    {
      path: "/",
      element: <Root />,
      errorElement: <ErrorPage />,
    },
    {
      path: "allocate",
      element: <App />,
      errorElement: <ErrorPage />,
    },
    {
      path: "about",
      element: <UnderConstructionPage />,
      errorElement: <ErrorPage />,
    },
    {
      path: "docs",
      element: <UnderConstructionPage />,
      errorElement: <ErrorPage />,
    },
  ]);

  return routes;
};
