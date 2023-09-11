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
          <h1 className="text-3xl font-bold">Page not found</h1>
          <span className="flex flex-col gap-y-2 items-center font-light">
            <p>But since we are here, enjoy a free Warren Buffet's quote:</p>
            <p className="italic">
              {
                "“If you aren't willing to own a stock for 10 years, don't even think about owning it for 10 minutes”"
              }
            </p>
          </span>
        </div>
        <Footer />
      </div>
    </>
  );
}
