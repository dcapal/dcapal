import React from "react";
import { NavBar } from "@components/core/navBar";
import { DcaPalHelmet } from "./helmet";
import { Footer } from "@components/core/footer";
import { CookieButton } from "@components/core/cookieButton";
import CookieConsent from "@components/core/cookieConsent";

export const ContainerPage = ({ id, title, content }) => {
  return (
    <>
      <DcaPalHelmet id={id} title={title} />
      <div className="w-full h-screen">
        <div className="h-full flex flex-col items-center">
          <NavBar />
          {content}
          <Footer />
          {Number(process.env.REACT_APP_ENABLE_COOKIE_BUTTON) === 1 && (
            <CookieButton />
          )}
        </div>
        {Number(process.env.REACT_APP_ENABLE_COOKIE_BUTTON) === 1 && (
          <CookieConsent />
        )}
      </div>
    </>
  );
};
