import React, { useRef } from "react";
import { useDispatch } from "react-redux";
import { setAllocationFlowStep, setPfolioFile, Step } from "../../app/appSlice";
import { useTranslation } from "react-i18next";

export const InitStep = () => {
  const inputPfolio = useRef(null);
  const dispatch = useDispatch();

  const { t } = useTranslation();
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
        dispatch(setPfolioFile({ file: event.target.result }));
        dispatch(setAllocationFlowStep({ step: Step.IMPORT }));
      };
      reader.readAsText(e.target.files[0]);
    }
  };

  return (
    <div className="w-full h-full flex flex-col items-center">
      <div
        className="py-2 px-3 mt-6 mb-4 flex justify-center items-center cursor-pointer bg-neutral-500 hover:bg-neutral-600 active:bg-neutral-800 text-white text-2xl rounded shadow-md"
        onClick={nextStep}
      >
        {t("initStep.newPortfolio")}
      </div>
      <div className="text-lg">
        <span className="text-2xl">📤</span>
        {t("initStep.loadYourPortfolio")}{" "}
        <span
          className="font-medium underline text-[blue] cursor-pointer"
          onClick={onClickUpload}
        >
          {t("initStep.fromFile")}
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
