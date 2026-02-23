import React from "react";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { TransactionFees } from "./transactionFees";
import { SlidersHorizontal } from "lucide-react";
import { useTranslation } from "react-i18next";

export const PreferencesDialog = () => {
  const { t } = useTranslation();

  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button variant="outline" data-testid="portfolio.preferences.button">
          <SlidersHorizontal />
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-[512px]" data-testid="portfolio.preferences.dialog">
        <DialogHeader>
          <DialogTitle>{t("portfolioStep.preferences")}</DialogTitle>
          <DialogDescription>
            {t("portfolioStep.preferencesDescription")}
          </DialogDescription>
        </DialogHeader>
        <div className="py-2 flex flex-col gap-2 justify-center">
          <h3 className="scroll-m-20 text-md font-medium tracking-tight">
            {t("portfolioStep.transactionFees")} 💸
          </h3>
          <TransactionFees />
        </div>
      </DialogContent>
    </Dialog>
  );
};
