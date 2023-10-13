import React, { useEffect } from "react";
import { useDispatch, useSelector } from "react-redux";
import { useLocation, useRoutes } from "react-router-dom";
import { App } from "../app";
import { setAllocationFlowStep, Step } from "../app/appSlice";

import AboutPage from "./aboutPage";
import DemoPage from "./demoPage";
import ErrorPage from "./errorPage";
import NotFoundPage from "./notFoundPage";
import { Root } from "./root";
import UnderConstructionPage from "./underConstruction";

import {
  DEMO_PF_60_40,
  DEMO_PF_ALL_SEASONS,
  DEMO_PF_MR_RIP,
} from "../app/config";
import { ContainerPage } from "./containerPage";

export const Router = () => {
  const step = useSelector((state) => state.app.allocationFlowStep);

  const dispatch = useDispatch();
  const location = useLocation();

  useEffect(() => {
    if (location.pathname === "/allocate" && step > Step.PORTFOLIO) {
      dispatch(setAllocationFlowStep({ step: Step.PORTFOLIO }));
    }
  }, [location]);

  let routesConfig = [
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
      element: <UnderConstructionPage />,
      errorElement: <ErrorPage />,
    },
  ];

  const demoRoutes = [DEMO_PF_60_40, DEMO_PF_ALL_SEASONS, DEMO_PF_MR_RIP];
  for (const route of demoRoutes) {
    routesConfig.push({
      path: `demo/${route}`,
      element: <DemoPage />,
      errorElement: <ErrorPage />,
    });
  }

  return useRoutes(routesConfig);
};
