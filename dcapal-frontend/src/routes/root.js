import React, { useEffect, useRef } from "react";
import { NavBar } from "../components/core/navBar";

import FrontImage from "../../images/investing_front.svg";
import { useNavigate } from "react-router-dom";
import { useDispatch } from "react-redux";
import { setAllocationFlowStep, setPfolioFile, Step } from "../app/appSlice";

export const Root = () => {
  const inputPfolio = useRef(null);
  const dispatch = useDispatch();
  const navigate = useNavigate();

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
        navigate("/allocate");
      };
      reader.readAsText(e.target.files[0]);
    }
  };

  const onClickStart = () => {
    dispatch(setAllocationFlowStep({ step: Step.CCY }));
    navigate("/allocate");
  };

  return (
    <div className="w-full h-screen">
      <div className="flex flex-col h-full">
        <NavBar />
        <div className="w-full px-4 py-8 bg-[#ededed]">
          <div className="flex flex-col items-center gap-y-7">
            <div className="w-full max-w-[35rem] flex flex-col items-start gap-y-7">
              <p className="text-3xl sm:text-4xl font-bold">
                Dollar Cost Averaging made easy
              </p>
              <p className="text-xl sm:text-2xl font-light">
                DcaPal helps you keep your portfolio under control with
                tax-efficient suggestions for your monthly investments.
              </p>
            </div>
            <img className="w-full px-4 max-w-[26rem]" src={FrontImage} />
            <div className="w-full max-w-[26rem] flex gap-x-4 justify-center">
              <div>
                <button
                  className="px-3 py-2 flex justify-center items-center border border-neutral-500 hover:bg-neutral-600 active:bg-neutral-800 text-black hover:text-white text-lg rounded"
                  onClick={onClickUpload}
                >
                  Import portfolio
                </button>
                <input
                  style={{ display: "none" }}
                  type="file"
                  accept=".json"
                  ref={inputPfolio}
                  onChange={onChangeInputPfolio}
                />
              </div>
              <button
                className="px-3 py-2 flex justify-center items-center bg-neutral-500 hover:bg-neutral-600 active:bg-neutral-800 text-white text-lg rounded"
                onClick={onClickStart}
              >
                Start from scratch
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
