import React from "react";

export const RootCard = ({ imgSrc, text, ...props }) => {
  return (
    <div className="w-full min-h-[11.5rem] max-h-[11.5rem] flex flex-col gap-y-2 items-center justify-center bg-[#ededed] rounded">
      <img
        src={imgSrc}
        className="w-full max-w-[4rem] p-1 rounded-full bg-neutral-600"
      />
      <p className="w-4/5 text-xl text-center font-light">{text}</p>
    </div>
  );
};
