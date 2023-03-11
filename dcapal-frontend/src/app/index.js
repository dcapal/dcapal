import React, { useEffect } from "react";
import { useDispatch } from "react-redux";

import { AllocationFlow } from "../components/allocationFlow";
import { Footer } from "../components/core/footer";
import { NavBar } from "../components/core/navBar";
import { DcaPalHelmet } from "../routes/helmet";
import { setCurrencies } from "./appSlice";
import { fetchAssetsDcaPal } from "./providers";

const loadCurrencies = async (dispatch) => {
  const res = await fetchAssetsDcaPal("fiat");
  const ccys = res.map((c) => c.symbol);

  dispatch(setCurrencies({ currencies: ccys }));
};

const Main = () => <AllocationFlow />;

export const App = () => {
  const dispatch = useDispatch();

  useEffect(() => {
    loadCurrencies(dispatch);
  }, []);

  return (
    <>
      <DcaPalHelmet title={"Allocate"} />
      <div className="relative w-full h-screen">
        <div className="absolute bg-[#ededed] w-full h-[50px] top-10 -z-40" />
        <div className="absolute app-bg -z-50" />
        <div className="flex flex-col h-full">
          <NavBar />
          <div className="flex flex-col h-full px-6 pt-4">
            <div className="w-full max-w-[42rem] pb-6 grow self-center">
              <Main />
            </div>
          </div>
          <Footer />
        </div>
      </div>
    </>
  );
};
