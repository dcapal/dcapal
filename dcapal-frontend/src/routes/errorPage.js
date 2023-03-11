import React from "react";

import { useRouteError } from "react-router-dom";
import { Footer } from "../components/core/footer";
import { NavBar } from "../components/core/navBar";
import { DcaPalHelmet } from "./helmet";

export default function ErrorPage() {
  const error = useRouteError();
  console.error(error);

  return (
    <>
      <DcaPalHelmet title="Error" />
      <div className="w-full h-screen flex flex-col">
        <NavBar />
        <div className="flex flex-col grow justify-center items-center gap-10 text-center">
          <h1 className="text-4xl font-bold">Oops!</h1>
          <p>Sorry, an unexpected error has occurred.</p>
          <p className="font-light">
            <i>{error.statusText || error.message}</i>
          </p>
        </div>
        <Footer />
      </div>
    </>
  );
}
