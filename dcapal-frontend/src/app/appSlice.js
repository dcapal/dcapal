import { createSlice } from "@reduxjs/toolkit";
import { Step } from "../components/allocationFlow";

export const appSlice = createSlice({
  name: "app",
  initialState: {
    allocationFlowStep: Step.INIT,
    currencies: [],
  },
  reducers: {
    setAllocationFlowStep: (state, action) => {
      state.allocationFlowStep = action.payload.step;
    },
    setCurrencies: (state, action) => {
      state.currencies = action.payload.currencies;
    },
  },
});

export const { setAllocationFlowStep, setCurrencies } = appSlice.actions;

export default appSlice.reducer;
