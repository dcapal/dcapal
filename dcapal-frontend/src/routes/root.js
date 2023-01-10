import React, { useEffect, useRef } from "react";
import { NavBar } from "../components/core/navBar";

import FrontImage from "../../images/investing_front.svg";
import PortfolioLogo from "../../images/icons/portfolio.svg";
import AmountLogo from "../../images/icons/amount.svg";
import RebalanceLogo from "../../images/icons/rebalance.svg";
import MarketLogo from "../../images/icons/market.svg";

import { useNavigate } from "react-router-dom";
import { useDispatch } from "react-redux";
import { setAllocationFlowStep, setPfolioFile, Step } from "../app/appSlice";
import { LaunchBtn } from "../components/core/launchBtn";
import { RootCard } from "../components/core/rootCard";

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
      <div className="flex flex-col h-full items-center">
        <NavBar />
        <div className="w-full px-4 py-8 mb-8 bg-[#ededed]">
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
            <LaunchBtn />
          </div>
        </div>
        <div className="w-full max-w-[38rem] px-4 flex flex-col gap-y-5 text-center">
          <p className="w-full text-3xl font-semibold">
            Your smart assistant to know how much to invest on each asset every
            month
          </p>
          <span className="w-full flex flex-col text-xl font-light gap-y-2">
            <p>
              Keeping your portfolio well-balanced is tough. We know how it is.
            </p>
            <p>
              You do your asset allocation on day one, then market moves and you
              get lost on how to split your monthly savings.
            </p>
            <p>DcaPal takes care of that for you.</p>
          </span>
        </div>
        <div className="w-full max-w-[25rem] px-4 pt-8 flex flex-col gap-y-6">
          <RootCard
            imgSrc={PortfolioLogo}
            text={
              <p>
                <span className="font-normal">Build your portfolio</span> and
                define your asset allocation
              </p>
            }
          />
          <RootCard
            imgSrc={AmountLogo}
            text={
              <p>
                Tell us <span className="font-normal">your budget</span> for
                this month
              </p>
            }
          />
          <RootCard
            imgSrc={RebalanceLogo}
            text={
              <p>
                Choose between
                <span className="font-normal"> tax-efficient</span> or standard
                <span className="font-normal"> rebalancing</span>
              </p>
            }
          />
          <RootCard
            imgSrc={MarketLogo}
            text={
              <p>
                Learn how much to invest on each asset and
                <span className="font-normal"> go to market</span>
              </p>
            }
          />
        </div>
        <div className="w-full px-4 py-8 flex justify-center">
          <LaunchBtn />
        </div>
      </div>
    </div>
  );
};
