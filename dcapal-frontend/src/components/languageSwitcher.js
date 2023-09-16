import React from "react";
import { useTranslation } from "react-i18next";
import { setLanguage } from "../app/appSlice";
import { useDispatch, useSelector } from "react-redux";
import i18n from "i18next";
function LanguageSwitcher() {
  const { i18n, t } = useTranslation();
  const dispatch = useDispatch();

  return (
    <select
      className="pl-3 pr-1.5 py-2 pb-2 border focus-visible:outline-1 rounded-md text-center border-gray-300 hover:border-gray-500 focus-visible:outline-gray-600 leading-normal"
      value={
        useSelector((state) => state.app.language) || i18n.resolvedLanguage
      }
      onChange={(e) => {
        const newLang = e.target.value;
        i18n.changeLanguage(newLang).then(() => {
          dispatch(setLanguage({ language: newLang || i18n.resolvedLanguage }));
        });
      }}
    >
      <option value="en"> {t("language.en")} </option>
      <option value="it"> {t("language.it")} </option>
    </select>
  );
}
export default LanguageSwitcher;
