import React, { useEffect, useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import classNames from "classnames";
import { useMediaQuery } from "@react-hook/media-query";

import toast from "react-hot-toast";

import { PDFDownloadLink } from "@react-pdf/renderer";
import { SearchBar } from "./searchBar";
import { AssetCard } from "./assetCard";
import { PortfolioSummaryDocument } from "./documentSummary";

import {
  addAsset,
  currentPortfolio,
  selectPortfolio,
  setPrice,
  setQty,
  setRefreshTime,
  setTargetWeight,
} from "@components/allocationFlow/portfolioSlice";

import { setAllocationFlowStep, Step } from "@app/appSlice";

import { MEDIA_SMALL, REFRESH_PRICE_INTERVAL_SEC } from "@app/config";

import BAG from "@images/icons/bag.svg";
import PIECHART from "@images/icons/piechart.svg";
import PDF from "@images/icons/pdf-document.svg";
import { getFetcher } from "@app/providers";
import { Trans, useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { PreferencesDialog } from "./preferencesDialog";

const refreshAssetPrices = async (assets, quoteCcy, validCcys, dispatch, t) => {
  console.debug("Refreshing prices (", new Date(), ")");

  if (Object.keys(assets) < 1) {
    dispatch(setRefreshTime({ time: Date.now() }));
    return;
  }

  Object.values(assets).forEach(async (a) => {
    const price = await getFetcher(a.provider, validCcys)(a.symbol, quoteCcy);
    if (!price) {
      console.warn(
        "[ImportStep] Failed to fetch price for",
        a.symbol,
        `(provider=${a.provider} quoteCcy=${quoteCcy})`
      );
      return;
    }
    dispatch(setPrice({ symbol: a.symbol, price: price }));
  });

  toast.success(t("common.refreshedPrices"));
  dispatch(setRefreshTime({ time: Date.now() }));
};

export const PortfolioStep = () => {
  const [searchText, setSearchText] = useState("");

  const { t, i18n } = useTranslation();
  const pfolioName = useSelector((state) => currentPortfolio(state).name);
  const assetStore = useSelector((state) => currentPortfolio(state).assets);
  const quoteCcy = useSelector((state) => currentPortfolio(state).quoteCcy);
  const validCcys = useSelector((state) => state.app.currencies);
  const lastRefreshTime = useSelector(
    (state) => currentPortfolio(state).lastPriceRefresh
  );

  const isMobile = !useMediaQuery(MEDIA_SMALL);
  const dispatch = useDispatch();

  useEffect(() => {
    let timeout = null;

    const refreshPrices = async () => {
      const now = new Date();
      const nextRefresh = new Date(
        lastRefreshTime + REFRESH_PRICE_INTERVAL_SEC * 1000
      );

      if (now > nextRefresh) {
        await refreshAssetPrices(assetStore, quoteCcy, validCcys, dispatch, t);
        return;
      }

      timeout = setTimeout(async () => {
        await refreshAssetPrices(assetStore, quoteCcy, validCcys, dispatch, t);
      }, nextRefresh - now);
    };

    refreshPrices().catch(console.error);

    return () => {
      if (timeout) clearTimeout(timeout);
    };
  }, [lastRefreshTime]);

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

  const onClickGoBack = () => {
    dispatch(selectPortfolio({ id: null }));
    dispatch(setAllocationFlowStep({ step: Step.PORTFOLIOS }));
  };

  const onClickAddLiquidity = () => {
    dispatch(setAllocationFlowStep({ step: Step.INVEST }));
  };

  return (
    <div className="w-full flex flex-col pt-2 items-center">
      <div className="w-full my-2">
        <SearchBar
          text={searchText}
          setText={setSearchText}
          addAsset={addAssetToPortfolio}
        />
      </div>
      {assets && assets.length > 0 && (
        <div className="relative w-full flex flex-col items-end justify-center mt-2">
          <PreferencesDialog />
        </div>
      )}
      {assets && assets.length > 0 && (
        <div className="w-full flex items-center mb-3 pl-3 font-light text-2xl">
          {pfolioName || t("importStep.defaultPortfolioName")}
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
          onClick={onClickGoBack}
        >
          {t("common.goBack")}
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
            <img
              className="w-full max-w-[3rem] p-1 self-start"
              alt="Bag"
              src={BAG}
            />
            <p className="flex-grow font-light">
              <Trans
                i18nKey="portfolioStep.fillWithNumber"
                values={{
                  field: t("portfolioStep.quantity"),
                  symbol: assets[assets.length - 1].symbol,
                }}
                components={[
                  <span className="font-normal" />,
                  <span className="uppercase" />,
                ]}
              />
            </p>
          </div>
          <div className="w-full flex items-center justify-start">
            <img
              className="w-full max-w-[3rem] p-1 self-start"
              alt="Piechart"
              src={PIECHART}
            />
            <p className="flex-grow font-light">
              <Trans
                i18nKey="portfolioStep.defineTargetWeight"
                values={{
                  targetWeight: t("portfolioStep.targetWeight"),
                  percentage: "20%",
                }}
                components={[
                  <span className="font-normal" />,
                  <span className="italic" />,
                ]}
              />
            </p>
          </div>
          {assets && assets.length > 0 && (
            <div className="w-full flex items-center justify-start">
              <img
                className="w-full max-w-[3rem] p-1 self-start"
                alt="PDF"
                src={PDF}
              />
              <PDFDownloadLink
                document={<PortfolioSummaryDocument assets={assets} />}
                fileName={`${pfolioName}_${new Date().toISOString().slice(0, 10)}.pdf`}
              >
                {({ loading }) =>
                  loading ? (
                    "Loading document..."
                  ) : (
                    <p className="flex-grow font-light">
                      <Trans
                        i18nKey="portfolioStep.downloadDocument"
                        components={[
                          <span className="font-normal" />,
                          <span className="italic" />,
                        ]}
                      />
                    </p>
                  )
                }
              </PDFDownloadLink>
            </div>
          )}
        </div>
      )}
      {(isFirstCardFilled || Object.keys(assetStore).length > 1) &&
        !isAllAllocated && (
          <div className="mt-6 font-light text-red-500">
            <Trans
              i18nKey="portfolioStep.reviewYourWeight"
              values={{
                targetWeights: t("portfolioStep.targetWeights"),
                actualWeight: cumWeight.toLocaleString(i18n.language, {
                  maximumFractionDigits: 12,
                }),
              }}
              components={[
                <span className="font-normal" />,
                <span className="font-normal" />,
              ]}
            />
          </div>
        )}
      {Object.keys(assetStore).length > 0 && (
        <>
          <p className="mt-6 font-thin text-xs">
            {t("portfolioStep.lastFetch")}{" "}
            {new Date(lastRefreshTime).toLocaleString(i18n.language)}
          </p>
          <div className="w-full mt-6 flex justify-between items-center">
            <Button variant="link" size="link" onClick={onClickGoBack}>
              {t("common.goBack")}
            </Button>
            <Button onClick={onClickAddLiquidity} disabled={!isAllAllocated}>
              {t("portfolioStep.confirmWeights")}
            </Button>
          </div>
        </>
      )}
    </div>
  );
};
