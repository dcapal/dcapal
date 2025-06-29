import React from 'react';
import { Document, Page, Text, View, StyleSheet } from '@react-pdf/renderer';

const styles = StyleSheet.create({
  page: { padding: 20, fontSize: 10 },
  section: { marginBottom: 10 },
  title: { fontSize: 18, marginBottom: 10, fontWeight: 'bold' },
  table: { width: '100%', borderWidth: 1, borderColor: '#000', borderStyle: 'solid' },
  tableRow: { flexDirection: 'row', borderBottomWidth: 1, borderColor: '#000' },
  tableHeader: { backgroundColor: '#eee' },
  cell: { padding: 4 },
  symbol: { width: '15%' },
  name: { width: '25%' },
  aclass: { width: '15%' },
  price: { width: '15%' },
  qty: { width: '10%' },
  weight: { width: '10%' },
  target: { width: '10%' },
});

export const PortfolioSummaryDocument = ({ assets }) => (
  <Document>
    <Page size="A4" style={styles.page}>
      <View style={styles.section}>
        <Text style={styles.title}>Portfolio Summary</Text>
        <View style={[styles.table, styles.tableHeader]}>
          <View style={[styles.tableRow, styles.tableHeader]}>
            <Text style={[styles.cell, styles.symbol]}>Symbol</Text>
            <Text style={[styles.cell, styles.name]}>Name</Text>
            <Text style={[styles.cell, styles.aclass]}>Class</Text>
            <Text style={[styles.cell, styles.price]}>Price</Text>
            <Text style={[styles.cell, styles.qty]}>Qty</Text>
            <Text style={[styles.cell, styles.weight]}>Weight</Text>
            <Text style={[styles.cell, styles.target]}>Target</Text>
          </View>
          {assets.map((a) => (
            <View style={styles.tableRow} key={a.symbol}>
              <Text style={[styles.cell, styles.symbol]}>{a.symbol}</Text>
              <Text style={[styles.cell, styles.name]}>{a.name}</Text>
              <Text style={[styles.cell, styles.aclass]}>{a.aclass}</Text>
              <Text style={[styles.cell, styles.price]}>{a.price}</Text>
              <Text style={[styles.cell, styles.qty]}>{a.qty}</Text>
              <Text style={[styles.cell, styles.weight]}>{a.weight}</Text>
              <Text style={[styles.cell, styles.target]}>{a.targetWeight}</Text>
            </View>
          ))}
        </View>
      </View>
    </Page>
  </Document>
);
