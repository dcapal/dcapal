import React from "react";
import { Link } from "react-router-dom";

export const Footer = () => {
  return (
    <footer className="w-full px-12 pt-7 pb-10 flex flex-col gap-y-4 justify-between items-center bg-[#333333] text-white font-light">
      <div className="w-full flex flex-wrap gap-x-5 gap-y-2 justify-center text-xs">
        <a href="https://raw.githubusercontent.com/leonardoarcari/dcapal/master/LICENSE">
          Terms
        </a>
        <Link to={"/about#privacy-policy"}>Privacy</Link>
        <Link to={"/about#social-profiles"}>Contacts</Link>
        <Link to={"/docs"}>Docs</Link>
        <Link to={"/about"}>About</Link>
        <a href="https://github.com/leonardoarcari/dcapal">Github</a>
      </div>
      <p className="text-xs text-white/60">© 2023 Leonardo Arcari</p>
    </footer>
  );
};
