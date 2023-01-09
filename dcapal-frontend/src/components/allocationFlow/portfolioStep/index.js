import React, { useState } from "react";
import { useSelector, useDispatch } from "react-redux";

import { SearchBar } from "./searchBar";
import { AssetCard } from "./assetCard";

import { addAsset, setQty, setTargetWeight } from "./portfolioSlice";
import { setAllocationFlowStep, Step } from "../../../app/appSlice";
import { useNavigate } from "react-router-dom";

export const PortfolioStep = ({ ...props }) => {
  const [searchText, setSearchText] = useState("");
  const assetStore = useSelector((state) => state.pfolio.assets);
  const dispatch = useDispatch();
  const navigate = useNavigate();

  const assets = Object.values(assetStore).sort((a, b) => a.idx - b.idx);
  let cumWeight = 0;
  const validity = assets.map((a) => {
    cumWeight += a.targetWeight;
    return cumWeight <= 100;
  });
  const isAllAllocated = cumWeight === 100;

  const addAssetToPortfolio = (asset) => {
    dispatch(
      addAsset({
        symbol: asset.symbol,
        name: asset.name,
        price: asset.price,
        baseCcy: asset.baseCcy,
        provider: asset.provider,
      })
    );
    setSearchText("");
  };

  const onClickDiscard = () => {
    navigate("/");
  };

  const onClickAddLiquidity = () => {
    dispatch(setAllocationFlowStep({ step: Step.INVEST }));
  };

  return (
    <div className="w-full h-full flex flex-col pt-2 items-center">
      <div className="w-full mt-2 mb-6">
        <SearchBar
          text={searchText}
          setText={setSearchText}
          addAsset={addAssetToPortfolio}
        />
      </div>
      <div className="w-full flex flex-col items-center">
        {assets.map((a, idx) => {
          const setAssetQty = (qty) => {
            dispatch(setQty({ symbol: a.symbol, qty: qty }));
          };

          const setAssetTargetWeight = (w) => {
            dispatch(setTargetWeight({ symbol: a.symbol, weight: w }));
          };

          return (
            <AssetCard
              key={a.symbol}
              symbol={a.symbol}
              name={a.name}
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
          className="font-medium underline cursor-pointer"
          onClick={onClickDiscard}
        >
          Go back
        </span>
      )}
      {Object.keys(assetStore).length > 0 && !isAllAllocated && (
        <div className="mt-6 text-red-500">
          Review your <span className="font-medium">Target Weights</span>. They
          must sum up to 100%
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
            className="px-3 py-2 flex justify-center items-center bg-neutral-500 hover:bg-neutral-600 active:bg-neutral-800 text-white text-lg rounded-md shadow-md disabled:pointer-events-none disabled:opacity-60"
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
