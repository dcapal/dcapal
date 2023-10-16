import React from "react";
import { NavBar } from "../components/core/navBar";
import { DcaPalHelmet } from "./helmet";
import { Footer } from "../components/core/footer";
import { CookieButton } from "../components/core/cookieButton";

export const ContainerPage = ({ id, title, content }) => {
  return (
    <>
      <DcaPalHelmet id={id} title={title} />
      <div className="w-full h-screen">
        <div className="flex flex-col h-full items-center">
          <NavBar />
          {content}
          <Footer />
          <CookieButton/>
        </div>
      </div>
    </>
  );
};
