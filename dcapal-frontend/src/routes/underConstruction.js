import React from "react";

import { NavBar } from "../components/core/navBar";

import UnderConstructionImage from "../../images/under-construction.svg";
import { Link } from "react-router-dom";

export default function UnderConstructionPage() {
  return (
    <div className="w-full h-screen flex flex-col">
      <NavBar />
      <div className="px-6 py-10 flex flex-col grow justify-center items-center text-center gap-8">
        <img
          className="w-full px-4 sm:max-w-[35rem] pb-2"
          src={UnderConstructionImage}
        />
        <h1 className="text-3xl font-bold">Under construction. Stay tuned!</h1>
        <span className="flex flex-col gap-y-2 items-center font-light">
          <p>Our engineers are working hard to get this page out soon.</p>
          <p>
            In the meantime,{" "}
            <span className="underline">
              <Link to={"/"}>enjoy our app!</Link>
            </span>
          </p>
        </span>
      </div>
    </div>
  );
}
