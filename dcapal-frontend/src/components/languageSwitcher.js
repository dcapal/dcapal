import React from "react";
import { useTranslation } from "react-i18next";
import { setLanguage, Step } from "../app/appSlice";
import { useDispatch, useSelector } from "react-redux";
function LanguageSwitcher() {
  const { i18n, t } = useTranslation();
  const dispatch = useDispatch();
  const storedLanguage = useSelector((state) => state.app.language);

  return (
    <select
      className="w-full w-fit px-3 py-2 pb-1.5 border focus-visible:outline-1 rounded-md text-center border-gray-300 hover:border-gray-500 focus-visible:outline-gray-600 leading-normal"
      value={storedLanguage || i18n.language}
      onChange={(e) => {
        i18n.changeLanguage(e.target.value);
        dispatch(setLanguage({ language: e.target.value }));
      }}
    >
      <option value="en"> {t("language.en")} </option>
      <option value="it"> {t("language.it")} </option>
    </select>
  );
}
export default LanguageSwitcher;
