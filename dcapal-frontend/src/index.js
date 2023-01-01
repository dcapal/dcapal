import React, { useEffect } from "react";
import { createRoot } from "react-dom/client";
import { store, persistor } from "./app/store";
import { Provider, useDispatch } from "react-redux";
import { Main } from "./app";

import "./style.css";
import { ExportBtn } from "./components/exportBtn";
import { PersistGate } from "redux-persist/integration/react";
import { fetchAssetsDcaPal } from "./app/providers";
import { setCurrencies } from "./app/appSlice";

const loadCurrencies = async (dispatch) => {
  const res = await fetchAssetsDcaPal("fiat");
  const ccys = res.map((c) => c.symbol);

  dispatch(setCurrencies({ currencies: ccys }));
};

const App = () => {
  const dispatch = useDispatch();

  useEffect(() => {
    loadCurrencies(dispatch);
  }, []);

  return (
    <div className="relative w-full h-screen">
      <div className="absolute bg-[#ededed] w-full h-[50px] top-10 -z-40" />
      <div className="absolute app-bg -z-50" />
      <div className="flex flex-col h-full">
        <div className="w-full h-14 min-h-[3.5rem] px-4 py-2 flex justify-between items-center bg-[#333333]">
          <div className="text-xl font-semibold text-white">DcaPal</div>
          <ExportBtn />
        </div>
        <div className="flex flex-col h-full px-6 pt-4">
          <div className="w-full max-w-[42rem] pb-6 grow self-center">
            <Main />
          </div>
        </div>
      </div>
    </div>
  );
};
const root = createRoot(document.getElementById("app"));
root.render(
  <Provider store={store}>
    <PersistGate loading={null} persistor={persistor}>
      <App />
    </PersistGate>
  </Provider>
);
