import { IKImage } from "imagekitio-react";
import React from "react";
import { IMAGEKIT_URL } from "../../app/config";

export const RootCard = ({ id, imgSrc, text, ...props }) => {
  return (
    <div
      id={id}
      className="w-full min-h-[11.5rem] max-h-[11.5rem] flex flex-col gap-y-2 items-center justify-center bg-[#ededed] rounded"
    >
      <IKImage
        className="w-full max-w-[4rem] p-1 rounded-full bg-neutral-600"
        urlEndpoint={IMAGEKIT_URL}
        path={imgSrc}
      />
      <p className="w-4/5 text-xl text-center font-light">{text}</p>
    </div>
  );
};
