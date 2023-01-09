import React, { useEffect } from "react";
import { useDispatch } from "react-redux";
import { useLocation } from "react-router-dom";
import { useRoutes } from "react-router-dom";
import { App } from "../app";
import { setAllocationFlowStep, setPfolioFile, Step } from "../app/appSlice";
import { clearPortfolio } from "../components/allocationFlow/portfolioStep/portfolioSlice";
import ErrorPage from "./errorPage";
import { Root } from "./root";

export const Router = () => {
  const dispatch = useDispatch();
  const location = useLocation();

  useEffect(() => {
    if (location.pathname !== "/allocate") {
      dispatch(clearPortfolio({}));
      dispatch(setAllocationFlowStep({ step: Step.INIT }));
      dispatch(setPfolioFile({ file: "" }));
    }
  }, [location]);

  const routes = useRoutes([
    {
      path: "/",
      element: <Root />,
      errorElement: <ErrorPage />,
    },
    {
      path: "allocate",
      element: <App />,
    },
  ]);

  return routes;
};
