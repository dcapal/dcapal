import { createSlice } from "@reduxjs/toolkit";

export const Step = Object.freeze({
  INIT: 0,
  CCY: 10,
  IMPORT: 20,
  PORTFOLIO: 30,
  INVEST: 40,
  END: 50,
});

export const appSlice = createSlice({
  name: "app",
  initialState: {
    allocationFlowStep: Step.CCY,
    currencies: [],
    pfolioFile: "",
    language: "en",
  },
  reducers: {
    setAllocationFlowStep: (state, action) => {
      state.allocationFlowStep = action.payload.step;
    },
    setCurrencies: (state, action) => {
      state.currencies = action.payload.currencies;
    },
    setPfolioFile: (state, action) => {
      state.pfolioFile = action.payload.file;
    },
    setLanguage: (state, action) => {
      state.language = action.payload.language;
    },
  },
});

export const {
  setAllocationFlowStep,
  setCurrencies,
  setPfolioFile,
  setLanguage,
} = appSlice.actions;

export default appSlice.reducer;
