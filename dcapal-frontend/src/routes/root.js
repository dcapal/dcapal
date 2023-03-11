import React from "react";
import { NavBar } from "../components/core/navBar";
import { IKImage } from "imagekitio-react";

import { LaunchBtn } from "../components/core/launchBtn";
import { RootCard } from "../components/core/rootCard";

import { IMAGEKIT_URL } from "../app/config";
import {
  HEADER_INVESTING_FRONT_SVG,
  ICON_AMOUNT_SVG,
  ICON_MARKET_SVG,
  ICON_PORTFOLIO_SVG,
  ICON_REBALANCE_SVG,
} from "../app/images";
import { DcaPalHelmet } from "./helmet";
import { Footer } from "../components/core/footer";

export const Root = () => {
  return (
    <>
      <DcaPalHelmet
        title={"DcaPal - A smart assistant for your periodic investments"}
      />
      <div className="w-full h-screen">
        <div className="flex flex-col h-full items-center">
          <NavBar />
          <div className="w-full px-6 py-8 mb-8 bg-[#ededed]">
            <div className="flex flex-col items-center gap-y-7">
              <div className="w-full max-w-[35rem] flex flex-col items-start gap-y-7">
                <h1 className="text-3xl sm:text-4xl font-bold">
                  Dollar Cost Averaging made easy
                </h1>
                <p className="text-xl sm:text-2xl font-light">
                  DcaPal helps you keep your portfolio under control with
                  tax-efficient suggestions for your monthly investments.
                </p>
              </div>
              <IKImage
                className="w-full px-4 sm:max-w-[26rem]"
                urlEndpoint={IMAGEKIT_URL}
                path={HEADER_INVESTING_FRONT_SVG}
              />
              <LaunchBtn />
            </div>
          </div>
          <div className="w-full max-w-[38rem] px-4 flex flex-col gap-y-5 text-center">
            <h2 className="w-full text-3xl font-semibold">
              Your smart assistant for allocating your monthly savings
            </h2>
            <span className="w-full flex flex-col text-xl font-light gap-y-2">
              <p>
                Keeping your portfolio well-balanced is tough.{" "}
                <span className="italic">We know how it is</span>.
              </p>
              <p>
                You do your asset allocation on day-one, market goes up and down
                and you get lost on how to split your monthly savings.
              </p>
              <p>DcaPal takes care of that for you.</p>
            </span>
          </div>
          <div
            id="allocate-process"
            className="w-full max-w-[25rem] px-4 pt-8 flex flex-col gap-y-6"
          >
            <RootCard
              imgSrc={ICON_PORTFOLIO_SVG}
              text={
                <p>
                  <span className="font-normal">Build your portfolio</span> and
                  define your asset allocation
                </p>
              }
            />
            <RootCard
              imgSrc={ICON_AMOUNT_SVG}
              text={
                <p>
                  Tell us <span className="font-normal">your budget</span> for
                  this month
                </p>
              }
            />
            <RootCard
              id={"tax-efficient"}
              imgSrc={ICON_REBALANCE_SVG}
              text={
                <p>
                  Choose
                  <span className="font-normal"> tax-efficient </span>
                  (buy-only) or standard
                  <span className="font-normal"> rebalancing</span>
                </p>
              }
            />
            <RootCard
              imgSrc={ICON_MARKET_SVG}
              text={
                <p>
                  Discover{" "}
                  <span className="font-normal">how much to invest </span>
                  on each asset and go to market
                </p>
              }
            />
          </div>
          <div className="w-full px-6 py-8 mt-10 bg-[#ededed]">
            <div className="flex flex-col items-center gap-y-7 text-center">
              <div className="w-full max-w-[35rem] flex flex-col gap-y-4">
                <p className="text-3xl font-semibold">
                  Get started now. It's free!
                </p>
                <p className="text-xl font-light">
                  We built DcaPal as quick tool for passive investors. Get your
                  allocation done in minutes.
                </p>
                <p className="text-xl font-light">No registration required!</p>
              </div>
              <LaunchBtn />
            </div>
          </div>
          <Footer />
        </div>
      </div>
    </>
  );
};
