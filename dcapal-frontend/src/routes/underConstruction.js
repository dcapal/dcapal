import React from "react";

import { NavBar } from "../components/core/navBar";

import { Link } from "react-router-dom";
import { IMAGEKIT_URL } from "../app/config";
import { IKImage } from "imagekitio-react";
import { HEADER_UNDER_CONSTRUCTION_SVG } from "../app/images";

export default function UnderConstructionPage() {
  return (
    <div className="w-full h-screen flex flex-col">
      <NavBar />
      <div className="px-6 py-10 flex flex-col grow justify-center items-center text-center gap-8">
        <IKImage
          className="w-full px-4 sm:max-w-[35rem] pb-2"
          urlEndpoint={IMAGEKIT_URL}
          path={HEADER_UNDER_CONSTRUCTION_SVG}
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
