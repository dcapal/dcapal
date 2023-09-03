import React, { useState } from "react";
import { useSelector, useDispatch } from "react-redux";
import classNames from "classnames";
import { useMediaQuery } from "@react-hook/media-query";

import { SearchBar } from "./searchBar";
import { AssetCard } from "./assetCard";

import {
  addAsset,
  clearPortfolio,
  setQty,
  setTargetWeight,
} from "./portfolioSlice";

import { setAllocationFlowStep, Step } from "../../../app/appSlice";
import { IKImage } from "imagekitio-react";
import { IMAGEKIT_URL, MEDIA_SMALL } from "../../../app/config";
import { ICON_BAG_SVG, ICON_PIECHART_SVG } from "../../../app/images";
import { TransactionFees } from "./transactionFees";

export const PortfolioStep = ({ ...props }) => {
  const [searchText, setSearchText] = useState("");
  const [isShowFees, setShowFees] = useState(false);
  const assetStore = useSelector((state) => state.pfolio.assets);
  const isMobile = !useMediaQuery(MEDIA_SMALL);
  const dispatch = useDispatch();

  const assets = Object.values(assetStore).sort((a, b) => a.idx - b.idx);
  let cumWeight = 0;
  const validity = assets.map((a) => {
    cumWeight += a.targetWeight;
    return cumWeight <= 100;
  });

  cumWeight = Math.round(cumWeight * 1e6) / 1e6;
  const isAllAllocated = cumWeight === 100;

  const isFirstCardFilled =
    assets && assets.length === 1 && assets[0].targetWeight > 0;

  const addAssetToPortfolio = (asset) => {
    dispatch(
      addAsset({
        symbol: asset.symbol,
        name: asset.name,
        aclass: asset.aclass,
        price: asset.price,
        baseCcy: asset.baseCcy,
        provider: asset.provider,
      })
    );
    setSearchText("");
  };

  const onClickDiscard = () => {
    dispatch(clearPortfolio({}));
    dispatch(setAllocationFlowStep({ step: Step.CCY }));
  };

  const onClickAddLiquidity = () => {
    dispatch(setAllocationFlowStep({ step: Step.INVEST }));
  };

  const onClickTransactionFees = () => {
    isShowFees ? setShowFees(false) : setShowFees(true);
  };

  const feeBtnClass = classNames(
    "px-3 pt-1.5 pb-2 z-30 flex justify-center items-center text-2xl font-medium rounded-md shadow-md border border-gray-300",
    {
      "bg-white hover:bg-neutral-600 hover:text-white hover:border-gray-600 active:bg-neutral-800":
        !isShowFees,
      "bg-neutral-600 text-white": isShowFees,
    }
  );

  return (
    <div className="w-full h-full flex flex-col pt-2 items-center">
      <div className="w-full my-2">
        <SearchBar
          text={searchText}
          setText={setSearchText}
          addAsset={addAssetToPortfolio}
        />
      </div>
      {assets && assets.length > 0 && (
        <div className="relative w-full flex flex-col items-end justify-center mt-2">
          <button className={feeBtnClass} onClick={onClickTransactionFees}>
            ⚙️
          </button>
          {isShowFees && (
            <div className="w-full max-w-lg relative -top-4 px-3 pt-2 pb-3 flex flex-col gap-2 bg-white shadow-md ring-1 ring-black/5 rounded-md">
              <p className="font-light text-2xl">💸 Transaction fees</p>
              <TransactionFees />
            </div>
          )}
        </div>
      )}
      {assets && assets.length > 0 && (
        <div className="w-full flex items-center mb-3 pl-3 font-light text-2xl">
          Portfolio assets
        </div>
      )}
      <div className="w-full flex flex-col items-center">
        {assets.map((a, idx) => {
          const setAssetQty = (qty) => {
            dispatch(setQty({ symbol: a.symbol, qty: qty }));
          };

          const setAssetTargetWeight = (w) => {
            dispatch(
              setTargetWeight({
                symbol: a.symbol,
                weight: w,
              })
            );
          };

          return (
            <AssetCard
              key={a.symbol}
              symbol={a.symbol}
              name={a.name}
              aclass={a.aclass}
              price={a.price}
              qty={a.qty}
              setQty={setAssetQty}
              weight={a.weight}
              targetWeight={a.targetWeight}
              setTargetWeight={setAssetTargetWeight}
              isValidTargetWeight={validity[idx]}
            />
          );
        })}
      </div>
      {Object.keys(assetStore).length === 0 && (
        <span
          className="mt-2 font-medium underline cursor-pointer"
          onClick={onClickDiscard}
        >
          Go back
        </span>
      )}
      {Object.keys(assetStore).length > 0 && (
        <div
          className={classNames("w-full max-w-[40rem] flex flex-col mt-4", {
            "gap-4": isMobile,
            "gap-2": !isMobile,
          })}
        >
          <div className="w-full flex items-center justify-start">
            <IKImage
              className="w-full max-w-[3rem] p-1 self-start"
              urlEndpoint={IMAGEKIT_URL}
              path={ICON_BAG_SVG}
            />
            <p className="flex-grow font-light">
              Fill <span className="font-normal">Quantity</span> field with the
              number of{" "}
              <span className="uppercase">
                {assets[assets.length - 1].symbol}
              </span>{" "}
              you already have in your portfolio (e.g. 10 units)
            </p>
          </div>
          <div className="w-full flex items-center justify-start">
            <IKImage
              className="w-full max-w-[3rem] p-1 self-start"
              urlEndpoint={IMAGEKIT_URL}
              path={ICON_PIECHART_SVG}
            />
            <p className="flex-grow font-light">
              Define your desired asset allocation in{" "}
              <span className="font-normal">Target weight</span> field (e.g.{" "}
              <span className="italic">20%</span> of total portfolio value)
            </p>
          </div>
        </div>
      )}
      {(isFirstCardFilled || Object.keys(assetStore).length > 1) &&
        !isAllAllocated && (
          <div className="mt-6 font-light text-red-500">
            Review your <span className="font-normal">Target Weights</span>.
            They must sum up to 100% (currently{" "}
            <span className="font-normal">
              {cumWeight.toLocaleString("en-US", {
                maximumFractionDigits: 12,
              })}
              %
            </span>
            )
          </div>
        )}
      {Object.keys(assetStore).length > 0 && (
        <div className="w-full mt-6 flex justify-between items-center">
          <span
            className="font-medium underline cursor-pointer"
            onClick={onClickDiscard}
          >
            Discard
          </span>
          <button
            className="px-3 pt-1.5 pb-2 flex justify-center items-center bg-neutral-500 hover:bg-neutral-600 active:bg-neutral-800 text-white text-lg rounded-md shadow-md disabled:pointer-events-none disabled:opacity-60"
            onClick={onClickAddLiquidity}
            disabled={!isAllAllocated}
          >
            Next
          </button>
        </div>
      )}
    </div>
  );
};
