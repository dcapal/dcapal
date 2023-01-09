import React from "react";
import { useNavigate } from "react-router-dom";
import { Link } from "react-router-dom";
import { ExportBtn } from "../exportBtn";

export const NavBar = () => {
  const navigate = useNavigate();

  const onClickHome = () => {
    navigate("/");
  };

  return (
    <div className="w-full h-14 min-h-[3.5rem] px-4 py-2 flex justify-between items-center bg-[#333333]">
      <div
        className="text-xl font-semibold text-white cursor-pointer"
        onClick={onClickHome}
      >
        <Link to={"/"}>DcaPal</Link>
      </div>
      <ExportBtn />
    </div>
  );
};
