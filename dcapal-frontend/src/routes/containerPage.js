import React from "react";
import { NavBar } from "../components/core/navBar";
import { DcaPalHelmet } from "./helmet";
import { Footer } from "../components/core/footer";

export const ContainerPage = ({ id, title, content }) => {
  return (
    <>
      <DcaPalHelmet id={id} title={title} />
      <div className="w-full h-screen">
        <div className="flex flex-col h-full items-center">
          <NavBar />
          {content}
          <Footer />
          <button
            type="button"
            style={{
              position: "fixed",
              bottom: 0,
              right: 0,
              top: "unset",
              zIndex: 3147483649,
              backgroundColor: "#ccc",
              padding: "5px 5px 0",
              borderTopLeftRadius: "0.5rem",
            }}
            data-cc="c-settings"
          >
            🍪
          </button>
        </div>
      </div>
    </>
  );
};
