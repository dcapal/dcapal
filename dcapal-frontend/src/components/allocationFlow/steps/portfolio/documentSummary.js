import React from "react";
import { Document, Page, Text, View, StyleSheet } from "@react-pdf/renderer";
import { useTranslation } from "react-i18next";

const styles = StyleSheet.create({
  page: { padding: 20, fontSize: 10 },
  section: { marginBottom: 10 },
  title: { fontSize: 18, marginBottom: 10, fontWeight: "bold" },
  table: {
    width: "100%",
    borderWidth: 1,
    borderColor: "#000",
    borderStyle: "solid",
  },
  tableRow: { flexDirection: "row", borderBottomWidth: 1, borderColor: "#000" },
  tableHeader: { backgroundColor: "#eee" },
  cell: { padding: 4 },
  symbol: { width: "12%" },
  name: { width: "18%" },
  price: { width: "12%" },
  abp: { width: "12%" },
  qty: { width: "10%" },
  weight: { width: "10%" },
  target: { width: "10%" },
  gain: { width: "10%" },
  gainPositive: { color: "#16a34a" },
  gainNegative: { color: "#dc2626" },
  summaryRow: { marginTop: 8, flexDirection: "row", justifyContent: "flex-end" },
});

export const PortfolioSummaryDocument = ({ assets }) => {
  const { t } = useTranslation();

  let totalCost = 0;
  let totalValue = 0;
  assets.forEach((a) => {
    if (a.qty > 0) {
      const effectiveABP = a.averageBuyPrice || a.price;
      totalCost += effectiveABP * a.qty;
      totalValue += a.price * a.qty;
    }
  });
  const portfolioGain =
    totalCost > 0 ? ((totalValue - totalCost) / totalCost) * 100 : null;

  return (
    <Document>
      <Page size="A4" style={styles.page}>
        <View style={styles.section}>
          <Text style={styles.title}>{t("pdf.name")}</Text>
          <View style={[styles.table, styles.tableHeader]}>
            <View style={[styles.tableRow, styles.tableHeader]}>
              <Text style={[styles.cell, styles.symbol]}>
                {t("pdf.symbol")}
              </Text>
              <Text style={[styles.cell, styles.name]}>
                {t("pdf.assetName")}
              </Text>
              <Text style={[styles.cell, styles.price]}>{t("pdf.price")}</Text>
              <Text style={[styles.cell, styles.abp]}>{t("pdf.abp")}</Text>
              <Text style={[styles.cell, styles.qty]}>{t("pdf.qty")}</Text>
              <Text style={[styles.cell, styles.weight]}>
                {t("pdf.weight")}
              </Text>
              <Text style={[styles.cell, styles.target]}>
                {t("pdf.target")}
              </Text>
              <Text style={[styles.cell, styles.gain]}>{t("pdf.gain")}</Text>
            </View>
            {assets.map((a) => {
              const effectiveABP = a.averageBuyPrice || a.price;
              const gain =
                a.qty > 0 && effectiveABP > 0
                  ? ((a.price - effectiveABP) / effectiveABP) * 100
                  : 0;
              const gainStyle =
                gain > 0
                  ? styles.gainPositive
                  : gain < 0
                    ? styles.gainNegative
                    : {};

              return (
                <View style={styles.tableRow} key={a.symbol}>
                  <Text style={[styles.cell, styles.symbol]}>{a.symbol}</Text>
                  <Text style={[styles.cell, styles.name]}>{a.name}</Text>
                  <Text style={[styles.cell, styles.price]}>
                    {Number(a.price).toFixed(2)}
                  </Text>
                  <Text style={[styles.cell, styles.abp]}>
                    {Number(effectiveABP).toFixed(2)}
                  </Text>
                  <Text style={[styles.cell, styles.qty]}>{a.qty}</Text>
                  <Text style={[styles.cell, styles.weight]}>
                    {Number(a.weight).toFixed(2)}
                  </Text>
                  <Text style={[styles.cell, styles.target]}>
                    {Number(a.targetWeight).toFixed(2)}
                  </Text>
                  <Text style={[styles.cell, styles.gain, gainStyle]}>
                    {a.qty > 0
                      ? `${gain > 0 ? "+" : ""}${gain.toFixed(2)}%`
                      : "-"}
                  </Text>
                </View>
              );
            })}
          </View>
          {portfolioGain !== null && (
            <View style={styles.summaryRow}>
              <Text
                style={
                  portfolioGain > 0
                    ? styles.gainPositive
                    : portfolioGain < 0
                      ? styles.gainNegative
                      : {}
                }
              >
                {t("portfolioStep.portfolioGain")}:{" "}
                {portfolioGain > 0 ? "+" : ""}
                {portfolioGain.toFixed(2)}% (MWR)
              </Text>
            </View>
          )}
        </View>
      </Page>
    </Document>
  );
};
