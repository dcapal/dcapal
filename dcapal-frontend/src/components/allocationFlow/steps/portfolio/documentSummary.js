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
  symbol: { width: "15%" },
  name: { width: "25%" },
  aclass: { width: "15%" },
  price: { width: "15%" },
  qty: { width: "10%" },
  weight: { width: "10%" },
  target: { width: "10%" },
});

export const PortfolioSummaryDocument = ({ assets }) => {
  const { t } = useTranslation();

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
              <Text style={[styles.cell, styles.qty]}>{t("pdf.qty")}</Text>
              <Text style={[styles.cell, styles.weight]}>
                {t("pdf.weight")}
              </Text>
              <Text style={[styles.cell, styles.target]}>
                {t("pdf.target")}
              </Text>
            </View>
            {assets.map((a) => (
              <View style={styles.tableRow} key={a.symbol}>
                <Text style={[styles.cell, styles.symbol]}>{a.symbol}</Text>
                <Text style={[styles.cell, styles.name]}>{a.name}</Text>
                <Text style={[styles.cell, styles.price]}>{a.price}</Text>
                <Text style={[styles.cell, styles.qty]}>{a.qty}</Text>
                <Text style={[styles.cell, styles.weight]}>
                  {Number(a.weight).toFixed(2)}
                </Text>
                <Text style={[styles.cell, styles.target]}>
                  {Number(a.targetWeight).toFixed(2)}
                </Text>
              </View>
            ))}
          </View>
        </View>
      </Page>
    </Document>
  );
};
