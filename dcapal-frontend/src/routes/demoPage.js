import React, { useEffect } from "react";
import { useDispatch } from "react-redux";
import { useLocation, useNavigate } from "react-router-dom";

import { DcaPalHelmet } from "./helmet";
import { NavBar } from "../components/core/navBar";
import { Footer } from "../components/core/footer";
import { clearPortfolio } from "../components/allocationFlow/portfolioStep/portfolioSlice";
import { setAllocationFlowStep, setPfolioFile, Step } from "../app/appSlice";

import {
  DEMO_PF_60_40,
  DEMO_PF_ALL_SEASONS,
  DEMO_PF_MR_RIP,
} from "../app/config";
import PF_60_40 from "../../demo/dcapal-60-40.json";
import PF_ALL_SEASONS from "../../demo/dcapal-all-seasons.json";
import PF_MR_RIP from "../../demo/dcapal-mrrip.json";

const demoPortfolios = {
  [DEMO_PF_60_40]: JSON.stringify(PF_60_40),
  [DEMO_PF_ALL_SEASONS]: JSON.stringify(PF_ALL_SEASONS),
  [DEMO_PF_MR_RIP]: JSON.stringify(PF_MR_RIP),
};

const getPfolioFile = (path) => {
  const demoId = path
    .split("/")
    .filter((e) => e !== "")
    .pop();

  if (demoId in demoPortfolios) {
    return demoPortfolios[demoId];
  }

  return "";
};

export default function DemoPage() {
  const navigate = useNavigate();
  const dispatch = useDispatch();
  const location = useLocation();

  const pfolioFile = getPfolioFile(location.pathname);

  useEffect(() => {
    if (pfolioFile && pfolioFile.length > 0) {
      dispatch(setPfolioFile({ file: pfolioFile }));
      dispatch(setAllocationFlowStep({ step: Step.IMPORT }));
    } else {
      dispatch(setPfolioFile({ file: "" }));
      dispatch(setAllocationFlowStep({ step: Step.CCY }));
    }

    dispatch(clearPortfolio({}));
    navigate("/allocate");
  }, [pfolioFile]);

  return (
    <>
      <DcaPalHelmet title="Demo - Mr. Rip" />
      <div className="w-full h-screen flex flex-col">
        <NavBar />
        <div className="px-6 py-10 flex flex-col grow justify-center items-center text-center gap-8">
          <h1 className="text-3xl font-bold">Loading demo portfolio</h1>
        </div>
        <Footer />
      </div>
    </>
  );
}
