import React, { useRef } from "react";
import { useDispatch } from "react-redux";
import { useNavigate } from "react-router-dom";
import { setAllocationFlowStep, setPfolioFile, Step } from "@app/appSlice";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";

export const LaunchBtn = () => {
  const inputPfolio = useRef(null);
  const dispatch = useDispatch();
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
        dispatch(setPfolioFile({ file: event.target.result }));
        dispatch(setAllocationFlowStep({ step: Step.IMPORT }));
        navigate("/portfolios");
      };
      reader.readAsText(e.target.files[0]);
    }
  };

  const onClickStart = () => {
    dispatch(setPfolioFile({ file: "" }));
    navigate("/portfolios");
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
