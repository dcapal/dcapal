import React from "react";
import { createRoot } from "react-dom/client";
import { createBrowserRouter, RouterProvider } from "react-router-dom";
import { Provider } from "react-redux";
import { PersistGate } from "redux-persist/integration/react";

import "./style.css";
import { App } from "./app";
import { store, persistor } from "./app/store";
import { Root } from "./routes/root";
import ErrorPage from "./routes/errorPage";
import { Router } from "./routes/router";
import { BrowserRouter } from "react-router-dom";

import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import LanguageDetector from "i18next-browser-languagedetector";

const router = createBrowserRouter([
  {
    path: "/",
    element: <Root />,
    errorElement: <ErrorPage />,
  },
  {
    path: "allocate",
    element: <App />,
  },
]);

i18n
  .use(LanguageDetector)
  .use(initReactI18next) // passes i18n down to react-i18next
  .init({
    // the translations
    // (tip move them in a JSON file and import them,
    // or even better, manage them via a UI: https://react.i18next.com/guides/multiple-translation-files#manage-your-translations-with-a-management-gui)
    resources: {
      en: {
        translation: {
          navbar: {
            getStarted: "Get Started",
            about: "About",
            docs: "Docs",
            exportPortfolio: "Export Portfolio",
          },
          currencyStep: {
            chooseCurrency: "Choose your portfolio currency",
            goBack: "Go back",
            next: "Next",
          },
          endStep: {
            leftBudget:
              "This is your budget left. Save it for the next time you visit DcaPal",
            buy: "Buy",
            sell: "Sell",
            nothingToTradeHold: "Nothing to trade. Just hold",
            nothingToTradeHodl: "Nothing to trade. Just hodl",
            currentAmount: "Current amount",
            newAmount: "New amount",
            currentWeight: "Current weight",
            newWeight: "New weight",
            targetWeight: "Target weight",
            fees: "Fees",
            feeImpact: "Fee impact",
            spacer: "Spacer",
            shouldHave: "Should have",
            butWouldHavePaid: "but would have paid",
            worthOfFees: "worth of fees",
            impact: "impact",
            budgetAllocated: "Hang on! We are allocating your budget",
            allocationReady: "Great Success! Your allocation is ready",
            backToPortfolio: "Back to Portfolio",
            opsBadHappened: "Oops! Something bad happened.",
            reviewPortfolio: "Please review your portfolio.",
          },
          assetCard: {
            price: "Price",
            quantity: "Quantity",
            targetWeight: "Target weight",
            transactionFees: "Transaction fees",
          },
          portfolioStep: {
            transactionFees: "Transaction fees",
            portfolioAssets: "Portfolio assets",
            goBack: "Go back",
            quantity: "Quantity",
            targetWeight: "Target weight",
            targetWeights: "Target weights",
            fillWithNumber:
              "Fill <0>{{field}}</0> field with the number of <1>{{symbol}}</1> you already have in your portfolio (e.g. 10 units)",
            defineTargetWeight:
              "Define your desired asset allocation in <0>{{targetWeight}}</0> field (e.g. <1>{{percentage}}</1> of total portfolio value)",
            reviewYourWeight:
              "Review your <0>{{targetWeights}}</0> They must sum up to 100% (currently <1>{{actualWeight}} %</1>)",
            lastFetch: "Prices last fetched at",
            discard: "Discard",
            next: "Next",
          },
          searchBar: {
            placeholder: "Search Crypto, ETF and much more",
            noAssetFoundFor: "No asset found for ",
            loading: "Loading",
          },
          transactionFee: {
            default: "Default",
            noFees: "No fees",
            fixed: "Fixed",
            variable: "Variable",
            zeroFee: "Enjoy your zero-fee trading life, lucky bastard",
            feeAmount: "Fee amount",
            feePercentage: "Fee percentage",
            minFee: "Min fee",
            maxFee: "Max fee",
            reviewFee:
              "Review your <0>{{fee}}</0>. Must be less than or equal to max fee.",
            maxFeeImpact: "Max fee impact",
          },
          importStep: {
            fetchData: "Just a sec! Fetching fresh data for your portfolio",
            importPortfolio: "Import Portfolio",
            ops: "Oops! This is embarassing",
            goBack: "Go back",
            allocateYourSavings: "Allocate your savings",
          },
          initStep: {
            newPortfolio: "New portfolio",
            loadYourPortfolio: "Or load your portfolio",
            fromFile: "from file",
          },
          investStep: {
            howMuchAllocate: "How much you would like to allocate?",
            taxEfficient: "Tax Efficient",
            taxEfficientAlgorithm: "Use <0>{{tax}}</0> algorithm",
            taxEfficientInfo:
              "With <0>{{tax}}</0> option, we do our best to rebalance your portfolio using your liquidity, with buy-only suggestions. Otherwise, we might suggest to sell part of your positions.",
            advanced: "Advanced",
            doNotSplit: "Don't split",
            doNotSplitInfo:
              "With <0>{{message}}</0> option, we are going to suggest the number of shares to buy or sell for non-fractional assets like Stocks or ETFs. Otherwise, we might suggest theoretically optimal amounts to buy or sell but you will need to figure out how many shares to buy.",
            wholeShares: "whole shares",
            goBack: "Go back",
            runAllocation: "Run Allocation",
          },
          about: {
            terms: "Terms",
            privacy: "Privacy",
            contacts: "Contacts",
            docs: "Docs",
            about: "About",
          },
          page: {
            notFound: {
              title: "Page not found",
              quote1:
                "But since we are here, enjoy a free Warren Buffet's quote:",
              quote2:
                "“If you aren't willing to own a stock for 10 years, don't even think about owning it for 10 minutes”",
            },
            error: {
              ops: "Oops!",
              errorHasOccurred: "Sorry, an unexpected error has occurred.",
            },
            demo: {
              loadingDemo: "Loading demo portfolio",
            },
            about: {
              title: "About",
              iAmLeo: "Hey there, I'm Leo!",
              pragmaticTool: "pragmatic tool",
              presentation1:
                "<0>{{person}}</0> 👋 I'm a Software Engineer in Finance, working on low-latency backend systems, mostly in C++ and Rust🦀. I know, I know, but there are worse things I could do.",
              presentation2:
                "I designed DcaPal as a <0>{{tool}}</0> for passive investors like me: financially-educated people managing their own portfolios of not-too-many assets replicating major world indices.",
              commonProblem:
                "I was facing a common problem: it's that time of the month, got some savings to invest and have to split them across my portfolio assets.",
              questionPresentation:
                "How the heck can I do it so that my portfolio stays balanced?",
              motivation:
                "Hence DcaPal. You come here every week/month/quarter, build your portfolio, define asset allocation in percentage, input how much you want to invest and <0>{{algorithm}}</0>",
              letTheAlgorithmDoTheSplittingForYou:
                "let the algorithm do the splitting for you",
              exportPortfolio:
                "You can even export your portfolio and import it back next time to save precious minutes.",
              privacyPolicyTitle: "Privacy policy",
              privacyPolicyMessage1:
                "Here's our poor man privacy policy. We store your IP location for stats purposes. That's it. Nothing more.",
              privacyPolicyMessage2:
                "Since DcaPal is a zero-login service, we don't do user profiling. We do not store any user data: portfolio assets, invested amounts, nothing ever leaves your browser.",
              privacyPolicyMessage3:
                "This may change in the future. Until then, long live the Far West.",
              privacyPolicyMessageTotal:
                "<0>{{part1}}</0><1>{{part2}}</1><2>{{part3}}</2>",
              feedback: "Feedback",
              feedbackMessage:
                " If you find any issue, would like to ask for new features or simply leave a feedback, <0>{{feelFree}}</0> to me on any of <1>{{socialProfiles}}</1>.",
              feelFreeReachOut: "feel free to reach out",
              socialProfiles: "my social profiles",
              everyContribution: "Every contribution is much appreciated!",
              fellowDev:
                "<alignSuper1>If you are a fellow Dev, don't forget to</alignSuper1> <githubStar>Star</githubStar> <alignSuper2>DcaPal on <githubLink>Github</githubLink> if you like it or drop me an</alignSuper2> <githubIssue>Issue</githubIssue> <alignSuper3>if you want to help.</alignSuper3>",
            },
            underConstruction: {
              title: "<0>Under construction. Stay tuned!</0>",
              message:
                "<container><p1>Our engineers are working hard to get this page out soon.</p1><p2>In the meantime, <underlineLink><linkHome>enjoy our app!</linkHome></underlineLink></p2></container>",
            },
            root: {
              title1: "Dollar Cost Averaging made easy",
              subtitle1:
                "DcaPal helps you keep your portfolio under control with tax-efficient suggestions for your monthly investments.",
              title2:
                "Your smart assistant for allocating your monthly savings",
              subtitle2:
                "<p1>Keeping your portfolio well-balanced is tough. <it>We know how it is</it>.</p1><p2>You do your asset allocation on day-one, market goes up and down and you get lost on how to split your monthly savings.</p2><p3>DcaPal takes care of that for you.</p3>",
              cardStep1:
                "<p1><bold>Build your portfolio</bold> and define your asset allocation</p1>",
              cardStep2:
                "<p1>Tell us <bold>your budget</bold> for this month</p1>",
              cardStep3:
                "<p1>Choose<bold1> tax-efficient </bold1> (buy-only) or standard <bold2> rebalancing</bold2></p1>",
              cardStep4:
                "<p1>Discover <bold>how much to invest </bold> on each asset and go to market</p1>",
              getStart:
                "<p1>Get started now. It's free!</p1><p2>We built DcaPal as quick tool for passive investors. Get your allocation done in minutes.</p2><p3>No registration required!</p3>",
            },
          },
        },
      },
      it: {
        translation: {},
      },
    },
    fallbackLng: "en",
    interpolation: {
      escapeValue: false, // react already safes from xss => https://www.i18next.com/translation-function/interpolation#unescape
    },
  });

const root = createRoot(document.getElementById("app"));
root.render(
  <React.StrictMode>
    <Provider store={store}>
      <PersistGate loading={null} persistor={persistor}>
        <BrowserRouter>
          <Router />
        </BrowserRouter>
      </PersistGate>
    </Provider>
  </React.StrictMode>
);
