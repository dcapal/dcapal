import React, { useRef } from "react";
import { useNavigate } from "react-router-dom";
import { Step, useAppStore } from "@/state/appStore";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";

export const LaunchBtn = () => {
  const inputPfolio = useRef(null);
  const setAllocationFlowStep = useAppStore(
    (state) => state.setAllocationFlowStep
  );
  const setPfolioFile = useAppStore((state) => state.setPfolioFile);
  const navigate = useNavigate();
  const { t } = useTranslation();
  const onClickUpload = () => {
    if (inputPfolio.current) {
      inputPfolio.current.click();
    }
  };

  const onChangeInputPfolio = (e) => {
    e.stopPropagation();
    e.preventDefault();
    if (e.target.files && e.target.files.length > 0) {
      const reader = new FileReader();
      reader.onload = function (event) {
        setPfolioFile({ file: event.target.result });
        setAllocationFlowStep({ step: Step.IMPORT });
        navigate("/allocate");
      };
      reader.readAsText(e.target.files[0]);
    }
  };

  const onClickStart = () => {
    setPfolioFile({ file: "" });
    setAllocationFlowStep({ step: Step.PORTFOLIOS });
    navigate("/allocate");
  };

  return (
    <div className="w-full flex flex-wrap-reverse sm:flex-wrap gap-x-4 gap-y-3 justify-center">
      <div className="min-w-full sm:min-w-0">
        <Button variant="outline" className="w-full" onClick={onClickUpload}>
          {t("importStep.importPortfolio")}
        </Button>
        <input
          style={{ display: "none" }}
          type="file"
          accept=".json"
          ref={inputPfolio}
          onChange={onChangeInputPfolio}
        />
      </div>
      <Button
        data-testid="importStep.allocateYourSavings"
        className="min-w-full sm:min-w-0"
        onClick={onClickStart}
      >
        {t("importStep.allocateYourSavings")}
      </Button>
    </div>
  );
};
