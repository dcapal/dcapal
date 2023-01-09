import React from "react";
import { useRoutes } from "react-router-dom";
import { App } from "../app";
import ErrorPage from "./errorPage";
import { Root } from "./root";

export const Router = () => {
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
