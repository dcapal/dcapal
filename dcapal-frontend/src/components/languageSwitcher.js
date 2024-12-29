import React from "react";
import { useTranslation } from "react-i18next";
import {
  Select,
  SelectTrigger,
  SelectContent,
  SelectValue,
  SelectItem,
} from "@/components/ui/select";

function LanguageSwitcher() {
  const { i18n, t } = useTranslation();

  return (
    <Select
      defaultValue={i18n.resolvedLanguage}
      onValueChange={(newLang) => {
        i18n.changeLanguage(newLang);
      }}
    >
      <SelectTrigger
        className="min-w-[4rem] w-auto"
        aria-label="Change language"
      >
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="en">{t("language.en")}</SelectItem>
        <SelectItem value="it">{t("language.it")}</SelectItem>
      </SelectContent>
    </Select>
  );
}
export default LanguageSwitcher;
