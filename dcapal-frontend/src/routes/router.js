import React from "react";
import { useRoutes } from "react-router-dom";
import { App } from "../app";
import AboutPage from "./aboutPage";
import ErrorPage from "./errorPage";
import NotFoundPage from "./notFoundPage";
import { Root } from "./root";
import UnderConstructionPage from "./underConstruction";

export const Router = () => {
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
  ]);

  return routes;
};
