import React, { lazy, useEffect, useState } from "react";
import { useRoutes } from "react-router-dom";

import AboutPage from "./aboutPage";
import DemoPage from "./demoPage";
import ErrorPage from "./errorPage";
import ImportPage from "./importPage";
import NotFoundPage from "./notFoundPage";
import { Root } from "./root";
import UnderConstructionPage from "./underConstruction";

import {
  DEMO_PF_60_40,
  DEMO_PF_ALL_SEASONS,
  DEMO_PF_HODLX,
  DEMO_PF_MR_RIP,
  supabase,
} from "@app/config";
import AuthPage from "@routes/loginPage";
import SignUpPage from "@routes/signUpPage";
import ResetPasswordPage from "@routes/resetPassword";
import { useSyncPortfolios } from "@hooks/useSyncPortfolios";

import(/* webpackPrefetch: true */ "@app");

const App = lazy(() => import("@app"));

export const Router = () => {
  const [session, setSession] = useState(null);

  useSyncPortfolios();

  useEffect(() => {
    const initSession = async () => {
      try {
        const {
          data: { session },
        } = await supabase.auth.getSession();
        setSession(session);
      } catch (error) {
        console.error("Error fetching session:", error);
      }
    };

    initSession();

    const {
      data: { subscription },
    } = supabase.auth.onAuthStateChange((_event, session) => {
      setSession(session);
    });

    return () => subscription.unsubscribe();
  }, []);

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
      path: "dashboard",
      element: <UnderConstructionPage />,
      errorElement: <ErrorPage />,
    },
    {
      path: "docs",
      element: <UnderConstructionPage />,
      errorElement: <ErrorPage />,
    },
    {
      path: "import",
      element: <ImportPage />,
      errorElement: <ErrorPage />,
    },
    {
      path: "login",
      element: <AuthPage />,
      errorElement: <ErrorPage />,
    },
    {
      path: "signup",
      element: <SignUpPage />,
      errorElement: <ErrorPage />,
    },
    {
      path: "reset-password",
      element: <ResetPasswordPage />,
      errorElement: <ErrorPage />,
    },
  ];

  const demoRoutes = [
    DEMO_PF_60_40,
    DEMO_PF_ALL_SEASONS,
    DEMO_PF_MR_RIP,
    DEMO_PF_HODLX,
  ];
  for (const route of demoRoutes) {
    routesConfig.push({
      path: `demo/${route}`,
      element: <DemoPage />,
      errorElement: <ErrorPage />,
    });
  }

  return useRoutes(routesConfig);
};
