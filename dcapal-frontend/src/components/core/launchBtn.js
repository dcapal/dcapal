import React, { useRef } from "react";
import { useDispatch } from "react-redux";
import { useNavigate } from "react-router-dom";
import { setAllocationFlowStep, setPfolioFile, Step } from "@app/appSlice";
import { useTranslation } from "react-i18next";
import { Box, Button, WrapItem } from "@chakra-ui/react";

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
        navigate("/allocate");
      };
      reader.readAsText(e.target.files[0]);
    }
  };

  const onClickStart = () => {
    dispatch(setPfolioFile({ file: "" }));
    dispatch(setAllocationFlowStep({ step: Step.PORTFOLIOS }));
    navigate("/allocate");
  };

  return (
    <Box
      display="flex"
      flexWrap={{ base: "wrap-reverse", sm: "wrap" }}
      columnGap="1rem"
      rowGap="0.75rem"
      justify="center"
      as="ul"
    >
      <WrapItem minW={{ base: "full", sm: "0px" }}>
        <Button size="lg" variant="outline" w="full" onClick={onClickUpload}>
          {t("importStep.importPortfolio")}
        </Button>
        <input
          style={{ display: "none" }}
          type="file"
          accept=".json"
          ref={inputPfolio}
          onChange={onChangeInputPfolio}
        />
      </WrapItem>
      <WrapItem minW={{ base: "full", sm: "0px" }}>
        <Button
          data-testid="importStep.allocateYourSavings"
          size="lg"
          w="full"
          onClick={onClickStart}
        >
          {t("importStep.allocateYourSavings")}
        </Button>
      </WrapItem>
    </Box>
  );
};
