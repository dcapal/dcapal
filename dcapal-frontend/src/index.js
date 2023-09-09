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

import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import LanguageDetector from 'i18next-browser-languagedetector';

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
                getStarted: 'Get Started',
                about: 'About',
                docs: 'Docs',
                exportPortfolio: 'Export Portfolio'
            },
              currencyStep: {
                  chooseCurrency: 'Choose your portfolio currency',
                  goBack: 'Go back',
                  next: 'Next',
              },
              endStep: {
                leftBudget: 'This is your budget left. Save it for the next time you visit DcaPal',
                buy: 'Buy',
                sell: 'Sell',
                  nothingToTradeHold: 'Nothing to trade. Just hold',
                  nothingToTradeHodl: 'Nothing to trade. Just hodl',
                  currentAmount: 'Current amount',
                  newAmount: 'New amount',
                  currentWeight: 'Current weight',
                  newWeight: 'New weight',
                  targetWeight: 'Target weight',
                  fees: 'Fees',
                  feeImpact: 'Fee impact',
                  spacer: 'Spacer',
                  shouldHave: 'Should have',
                  butWouldHavePaid: 'but would have paid',
                  worthOfFees: 'worth of fees',
                  impact: 'impact',
                  budgetAllocated: 'Hang on! We are allocating your budget',
                  allocationReady: 'Great Success! Your allocation is ready',
                  backToPortfolio: 'Back to Portfolio',
                  opsBadHappened: 'Oops! Something bad happened.',
                  reviewPortfolio: 'Please review your portfolio.',
              },
              assetCard: {
                price: 'Price',
                quantity: 'Quantity',
                targetWeight: 'Target weight',
                  transactionFees:'Transaction fees',
              },
              portfolioStep: {
                  transactionFees: "Transaction fees",
                  portfolioAssets: 'Portfolio assets',
                  goBack: 'Go back',
                  quantity: 'Quantity',
                  targetWeight: 'Target weight',
                  targetWeights: 'Target weights',
                  fillWithNumber: "Fill <0>{{field}}</0> field with the number of <1>{{symbol}}</1> you already have in your portfolio (e.g. 10 units)",
                    defineTargetWeight: 'Define your desired asset allocation in <0>{{targetWeight}}</0> field (e.g. <1>{{percentage}}</1> of total portfolio value)',
                  reviewYourWeight: 'Review your <0>{{targetWeights}}</0> They must sum up to 100% (currently <1>{{actualWeight}} %</1>)',
                    lastFetch: 'Prices last fetched at',
                  discard: 'Discard',
                  next: 'Next',
            },
              searchBar: {
                placeholder: "Search Crypto, ETF and much more",
                  noAssetFoundFor: 'No asset found for ',
                  loading: 'Loading',
              },
              transactionFee: {
                  default: 'Default',
                  noFees: "No fees",
                  fixed: "Fixed",
                  variable: "Variable",
                  zeroFee: 'Enjoy your zero-fee trading life, lucky bastard',
                  feeAmount: 'Fee amount',
                  feePercentage: 'Fee percentage',
                  minFee: 'Min fee',
                  maxFee: 'Max fee',
                  reviewFee: 'Review your <0>{{fee}}</0>. Must be less than or equal to max fee.',
                  maxFeeImpact: "Max fee impact",
              },
              importStep: {
                fetchData: 'Just a sec! Fetching fresh data for your portfolio',
                  importPortfolio: 'Import Portfolio',
                  ops: 'Oops! This is embarassing',
                  goBack: 'Go back',
                  allocateYourSavings: 'Allocate your savings',
              },
              initStep: {
                newPortfolio: 'New portfolio',
                  loadYourPortfolio: 'Or load your portfolio',
                  fromFile: 'from file',
              },
              investStep: {
                howMuchAllocate: 'How much you would like to allocate?',
                  taxEfficient: 'Tax Efficient',
                taxEfficientAlgorithm: 'Use <0>{{tax}}</0> algorithm',
                  taxEfficientInfo: 'With <0>{{tax}}</0> option, we do our best to rebalance your portfolio using your liquidity, with buy-only suggestions. Otherwise, we might suggest to sell part of your positions.',
                  advanced: 'Advanced',
                  doNotSplit: "Don't split",
                  doNotSplitInfo: "With <0>{{message}}</0> option, we are going to suggest the number of shares to buy or sell for non-fractional assets like Stocks or ETFs. Otherwise, we might suggest theoretically optimal amounts to buy or sell but you will need to figure out how many shares to buy.",
                  wholeShares: 'whole shares',
                  goBack: 'Go back',
                  runAllocation: 'Run Allocation',

              },
              about: {
                terms: 'Terms',
                  privacy: 'Privacy',
                  contacts: 'Contacts',
                  docs: 'Docs',
                  about: 'About',
              }
          }
        },
        it: {
          translation: {
          }
        }
      },
      fallbackLng: "en",
      interpolation: {
        escapeValue: false // react already safes from xss => https://www.i18next.com/translation-function/interpolation#unescape
      }
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
