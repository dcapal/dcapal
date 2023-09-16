import React from "react";

import { NavBar } from "../components/core/navBar";

import NOT_FOUND from "@images/headers/not-found.svg";

import { DcaPalHelmet } from "./helmet";
import { Footer } from "../components/core/footer";

export default function NotFoundPage() {
  return (
    <>
      <DcaPalHelmet title="Not Found" />
      <div className="w-full h-screen flex flex-col">
        <NavBar />
        <div className="px-6 py-10 flex flex-col grow justify-center items-center text-center gap-8">
          <img
            className="w-full px-4 sm:max-w-[35rem] pb-2"
            alt="Not found"
            src={NOT_FOUND}
          />
          <h1 className="text-3xl font-bold">{t("page.notFound.title")}</h1>
          <span className="flex flex-col gap-y-2 items-center font-light">
            <p>{t("page.notFound.quote1")}</p>
            <p className="italic">{t("page.notFound.quote2")}</p>
          </span>
        </div>
        <Footer />
      </div>
    </>
  );
}
