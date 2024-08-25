import React, { useEffect, useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import { spawn, Thread, Worker } from "threads";
import { setAllocationFlowStep, Step } from "@app/appSlice";
import { replacer, roundAmount, timeout } from "@utils/index.js";
import { Spinner } from "@components/spinner/spinner";
import {
  ACLASS,
  clearBudget,
  currentPortfolio,
  feeTypeToString,
  isWholeShares,
} from "@components/allocationFlow/portfolioSlice";
import { AllocateCard } from "./allocateCard";
import { useTranslation } from "react-i18next";
import { api } from "@app/api";
import { DCAPAL_API, supabase } from "@app/config";
import { useToast } from "@chakra-ui/react";

export const UNALLOCATED_CASH = "Unallocated cash";

const buildCards = (assets, solution, pfolioCcy, pfolioFees) => {
  const cards = Object.values(assets).map((a) => ({
    symbol: a.symbol,
    name: a.name,
    aclass: a.aclass,
    qty: -1,
    oldQty: a.qty,
    price: a.price,
    amount: 0,
    oldAmount: a.amount,
    weight: 0,
    oldWeight: a.weight,
    targetWeight: a.targetWeight,
    fees: a.fees ? a.fees : pfolioFees,
    theoAlloc: null,
    averageBuyPrice: a.abp,
  }));

  if (!solution?.amounts) return cards;

  const budgetLeft = solution.budget_left || 0;
  const hasCashPosition = pfolioCcy in assets;

  let totalAmount = hasCashPosition ? budgetLeft : 0;
  for (const a of solution.amounts.values()) {
    totalAmount += a;
  }

  for (const card of cards) {
    const isCashAsset = hasCashPosition && card.symbol === pfolioCcy;

    if (solution.amounts.has(card.symbol)) {
      card.amount = solution.amounts.get(card.symbol);
      card.amount += isCashAsset ? budgetLeft : 0;
      card.weight = totalAmount > 0 ? (100 * card.amount) / totalAmount : 0;
    }

    if (solution?.shares?.has(card.symbol) && !isCashAsset) {
      card.qty = solution.shares.get(card.symbol);
    }

    if (solution?.theo_allocs?.has(card.symbol)) {
      card.theoAlloc = solution.theo_allocs.get(card.symbol);
    }
  }

  if (budgetLeft && !hasCashPosition) {
    cards.push({
      symbol: pfolioCcy,
      name: UNALLOCATED_CASH,
      aclass: ACLASS.CURRENCY,
      qty: -1,
      oldQty: 0,
      price: 0,
      amount: budgetLeft,
      oldAmount: 0,
      weight: 0,
      oldWeight: 0,
      targetWeight: 0,
      fees: null,
      theo_alloc: null,
      averageBuyPrice: 0,
    });
  }

  return cards;
};

const buildFeesInput = (fees) => {
  if (!fees) {
    return null;
  }

  let input = {
    ...fees,
    feeStructure: {
      ...fees.feeStructure,
      type: feeTypeToString(fees.feeStructure.type),
    },
  };

  if (input.maxFeeImpact == null) {
    delete input.maxFeeImpact;
  } else if (input.maxFeeImpact) {
    input.maxFeeImpact /= 100;
  }

  if (input.feeStructure.feeRate == null) {
    delete input.feeStructure.feeRate;
  } else if (input.feeStructure.feeRate) {
    input.feeStructure.feeRate /= 100;
  }

  return input;
};

const buildProblemInput = (budget, assets, fees, useWholeShares) => {
  const as = Object.values(assets).reduce(
    (as, a) => ({
      ...as,
      [a.symbol]: {
        symbol: a.symbol,
        shares: a.qty,
        price: a.price,
        target_weight: a.targetWeight / 100,
        is_whole_shares: useWholeShares ? isWholeShares(a.aclass) : false,
        fees: buildFeesInput(a.fees),
      },
    }),
    {}
  );

  return [budget, as, buildFeesInput(fees)];
};

export const EndStep = ({ useTaxEfficient, useAllBudget, useWholeShares }) => {
  const [solution, setSolution] = useState(null);
  const [isLoading, setIsLoading] = useState(true);
  const dispatch = useDispatch();
  const toast = useToast();

  const { t } = useTranslation();
  const budget = useSelector((state) => currentPortfolio(state).budget);
  const assets = useSelector((state) => currentPortfolio(state).assets);
  const quoteCcy = useSelector((state) => currentPortfolio(state).quoteCcy);
  const fees = useSelector((state) => currentPortfolio(state).fees);
  const pfname = useSelector((state) => currentPortfolio(state).name);
  const pfid = useSelector((state) => currentPortfolio(state).id);

  const cards = solution ? buildCards(assets, solution, quoteCcy, fees) : [];

  useEffect(() => {
    const launchSolver = async () => {
      const solver = await spawn(
        new Worker(new URL("@workers/solver.js", import.meta.url), {
          name: "wasm-solver-worker",
        })
      );

      const [inputBudget, as, inputFees] = buildProblemInput(
        budget,
        assets,
        fees,
        useWholeShares
      );

      console.debug(
        `inputBudget=${inputBudget} as=${JSON.stringify(
          as
        )} quoteCcy=${quoteCcy} useTaxEfficient=${useTaxEfficient} useWholeShares=${useWholeShares} inputFees=${JSON.stringify(
          inputFees
        )}`
      );

      try {
        const sol = await solver.makeAndSolve(
          inputBudget,
          as,
          quoteCcy,
          inputFees,
          useTaxEfficient,
          useAllBudget
        );

        await Thread.terminate(solver);

        console.debug(`solution=${JSON.stringify(sol, replacer)}`);

        return sol;
      } catch (error) {
        console.error("Unexpected exception in dcapal-optimizer:", error);
        return null;
      }
    };

    const solve = async () => {
      const [sol] = await Promise.all([launchSolver(), timeout(1000)]);
      setIsLoading(false);
      if (sol) {
        setSolution(sol);
      }
    };

    solve();
  }, []);

  const [session, setSession] = useState(null);
  const [config, setConfig] = useState(null);

  useEffect(() => {
    supabase.auth.getSession().then(({ data: { session } }) => {
      setSession(session);
      setConfig({
        headers: { Authorization: `Bearer ${session.access_token}` },
      });
    });
    const {
      data: { subscription },
    } = supabase.auth.onAuthStateChange((_event, session) => {
      setSession(session);
    });
    return () => subscription.unsubscribe();
  }, []);

  const assetsArray = useSelector((state) =>
    Object.values(currentPortfolio(state).assets)
  );

  const [userData, setUserData] = useState(null);

  useEffect(() => {
    if (cards.length > 0) {
      setUserData({
        id: pfid,
        name: pfname,
        description: "optional",
        currency: quoteCcy,
        assets: cards
          .filter((card) => card.name !== UNALLOCATED_CASH)
          .map((a) => ({
            symbol: a.symbol,
            name: a.name,
            exchange: a.symbol,
            dataSource: a.provider,
            currency: a.currency,
            quantity: roundAmount(a.qty),
            price: roundAmount(a.price),
            averageBuyPrice: roundAmount(a.averageBuyPrice),
            weight: roundAmount(a.weight),
          })),
      });
    }
  }, [cards, pfid, pfname, quoteCcy]);

  const onClickSavePortfolio = async () => {
    if (!userData) {
      toast({
        title: "Error",
        description: "Portfolio data is not ready yet",
        status: "error",
        duration: 2000,
        isClosable: true,
      });
      return;
    }

    try {
      await api.post(`${DCAPAL_API}/v1/user/portfolios`, userData, config);
      toast({
        title: "Portfolio updated",
        status: "success",
        duration: 2000,
        isClosable: true,
      });
    } catch (error) {
      toast({
        title: "Error updating the portfolio",
        description: error.message,
        status: "error",
        duration: 2000,
        isClosable: true,
      });
    }
  };

  const onClickGoBack = () => {
    dispatch(clearBudget({}));
    dispatch(setAllocationFlowStep({ step: Step.PORTFOLIO }));
  };

  return (
    <div className="w-full flex flex-col items-center">
      {isLoading && (
        <>
          <div className="mt-2 mb-8 text-3xl font-light">
            {t("endStep.budgetAllocated")}
          </div>
          <Spinner />
        </>
      )}
      {!isLoading && solution && (
        <>
          <div className="mt-2 mb-8 text-3xl font-light">
            <span className="text-4xl">📊</span> {t("endStep.allocationReady")}
          </div>
          <div className="w-full flex flex-col items-center">
            {cards.map((c) => (
              <AllocateCard
                key={c.symbol}
                symbol={c.symbol}
                name={c.name}
                aclass={c.aclass}
                qty={c.qty}
                oldQty={c.oldQty}
                price={c.price}
                amount={roundAmount(c.amount)}
                oldAmount={roundAmount(c.oldAmount)}
                weight={c.weight}
                oldWeight={c.oldWeight}
                targetWeight={c.targetWeight}
                fees={c.fees}
                theoAlloc={c.theoAlloc}
              />
            ))}
          </div>
          <span
            className="mt-6 font-medium underline cursor-pointer"
            onClick={onClickSavePortfolio}
          >
            {t("endStep.savePortfolio")}
          </span>

          <span
            className="mt-6 font-medium underline cursor-pointer"
            onClick={onClickGoBack}
          >
            {t("endStep.backToPortfolio")}
          </span>
        </>
      )}
      {!isLoading && !solution && (
        <>
          <div className="mt-2 mb-8 text-3xl font-light">
            <span className="text-4xl">⚠️</span> {t("endStep.opsBadHappened")}
            {t("endStep.reviewPortfolio")}
          </div>
          <span
            className="mt-6 font-medium underline cursor-pointer"
            onClick={onClickGoBack}
          >
            {t("endStep.backToPortfolio")}
          </span>
        </>
      )}
    </div>
  );
};
