import React from "react";

export const RootCard = ({ id, imgSrc, text, ...props }) => {
  return (
    <div
      id={id}
      className="bg-card p-6 rounded-lg shadow-md  text-center bg-[#F3F4F6] rounded"
    >
      <div className="flex justify-center items-center">
        <img
          alt="Icon homepage"
          className="w-full max-w-[4rem] p-1 rounded-full bg-neutral-600"
          src={imgSrc}
        />
      </div>
      <p className="text-xl font-light mt-2">{text}</p>
    </div>
  );
};
