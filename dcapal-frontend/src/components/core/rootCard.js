import React from "react";

export const RootCard = ({ id, imgSrc, text, ...props }) => {
  return (
    <div
      id={id}
      className="bg-card p-6 rounded-lg shadow-md bg-[#F3F4F6] rounded text-center"
    >
      <div className="flex justify-center items-center">
        <img
          alt="Icon homepage"
          className="w-full max-w-[4rem] p-1 rounded-full bg-neutral-600"
          src={imgSrc}
        />
      </div>
      <p className="text-xl font-light mt-2">{text}</p>
      <div className="w-4/5 mx-auto text-center font-light"></div>
    </div>
  );
};
