import { useMediaQuery } from "@react-hook/media-query";
import React from "react";
import { useNavigate } from "react-router-dom";
import { Link } from "react-router-dom";
import { MEDIA_SMALL } from "../../app/config";
import { ExportBtn } from "../exportBtn";

export const NavBar = () => {
  const navigate = useNavigate();
  const isMobile = !useMediaQuery(MEDIA_SMALL);

  const onClickHome = () => {
    navigate("/");
  };

  return (
    <div className="w-full h-14 min-h-[3.5rem] px-4 py-2 flex justify-between items-center bg-[#333333]">
      <div className="flex gap-x-5">
        <div className="text-xl font-semibold text-white" onClick={onClickHome}>
          <Link to={"/"}>DcaPal</Link>
        </div>
        {!isMobile && (
          <>
            <div className="text-lg font-light text-white">
              <Link to={"/about"}>About</Link>
            </div>
            <div className="text-lg font-light text-white">
              <Link to={"/docs"}>Docs</Link>
            </div>
          </>
        )}
      </div>
      <ExportBtn />
    </div>
  );
};
