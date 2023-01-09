import React from "react";

import { useRouteError } from "react-router-dom";
import { NavBar } from "../components/core/navBar";

export default function ErrorPage() {
  const error = useRouteError();
  console.error(error);

  return (
    <div className="w-full h-screen flex flex-col">
      <NavBar />
      <div className="flex flex-col grow justify-center items-center gap-10">
        <h1 className="text-4xl font-bold">Oops!</h1>
        <p>Sorry, an unexpected error has occurred.</p>
        <p className="font-light">
          <i>{error.statusText || error.message}</i>
        </p>
      </div>
    </div>
  );
}
