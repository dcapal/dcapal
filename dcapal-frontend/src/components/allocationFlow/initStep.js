import React, { useRef } from "react";
import { useDispatch } from "react-redux";
import { Step } from ".";
import { setAllocationFlowStep } from "../../app/appSlice";

export const InitStep = ({ setPfolioFile, ...props }) => {
  const inputPfolio = useRef(null);
  const dispatch = useDispatch();

  const nextStep = () => {
    dispatch(setAllocationFlowStep({ step: Step.CCY }));
  };

  const onClickUpload = () => {
    if (inputPfolio.current) {
      inputPfolio.current.click();
    }
  };

  const onChangeInputPfolio = (e) => {
    e.stopPropagation();
    e.preventDefault();
    if (e.target.files && e.target.files.length > 0) {
      const reader = new FileReader();
      reader.onload = function (event) {
        setPfolioFile(event.target.result);
        dispatch(setAllocationFlowStep({ step: Step.IMPORT }));
      };
      reader.readAsText(e.target.files[0]);
    }
  };

  return (
    <div className="w-full h-full flex flex-col items-center">
      <div
        className="pt-1 pb-2 px-3 mt-6 mb-4 flex justify-center items-center cursor-pointer bg-neutral-500 hover:bg-neutral-600 active:bg-neutral-800 text-white text-2xl rounded shadow-md"
        onClick={nextStep}
      >
        New Portfolio
      </div>
      <div className="text-lg">
        <span className="text-2xl">📤</span>
        Or load your portfolio{" "}
        <span
          className="font-medium underline text-[blue] cursor-pointer"
          onClick={onClickUpload}
        >
          from file
        </span>
        <input
          style={{ display: "none" }}
          type="file"
          accept=".json"
          ref={inputPfolio}
          onChange={onChangeInputPfolio}
        />
      </div>
    </div>
  );
};
