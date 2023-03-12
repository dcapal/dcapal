import React, { useEffect } from "react";
import { useDispatch, useSelector } from "react-redux";
import { useLocation, useRoutes } from "react-router-dom";
import { App } from "../app";
import { setAllocationFlowStep, Step } from "../app/appSlice";
import { DEMO_PF_MR_RIP } from "../app/config";
import AboutPage from "./aboutPage";
import DemoPage from "./demoPage";
import ErrorPage from "./errorPage";
import NotFoundPage from "./notFoundPage";
import { Root } from "./root";
import UnderConstructionPage from "./underConstruction";

export const Router = () => {
  const step = useSelector((state) => state.app.allocationFlowStep);

  const dispatch = useDispatch();
  const location = useLocation();

  useEffect(() => {
    if (location.pathname === "/allocate" && step > Step.PORTFOLIO) {
      dispatch(setAllocationFlowStep({ step: Step.PORTFOLIO }));
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
      element: <AboutPage />,
      errorElement: <ErrorPage />,
    },
    {
      path: "docs",
      element: <UnderConstructionPage title="Docs" />,
      errorElement: <ErrorPage />,
    },
    {
      path: `demo/${DEMO_PF_MR_RIP}`,
      element: <DemoPage />,
      errorElement: <ErrorPage />,
    },
  ]);

  return routes;
};
