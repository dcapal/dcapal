import React from "react";
import {
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

function LinechartChart(props) {
  const data = [
    {
      name: "2018",
      uv: 1000,
    },
    {
      name: "2019",
      uv: 1700,
    },
    {
      name: "2020",
      uv: 2000,
    },
    {
      name: "2021",
      uv: 2780,
    },
    {
      name: "2022",
      uv: 1890,
    },
    {
      name: "2023",
      uv: 2390,
    },
    {
      name: "2024",
      uv: 3490,
    },
  ];

  return (
    <div {...props}>
      <ResponsiveContainer width="100%" height={600}>
        <LineChart
          width={500}
          height={200}
          data={data}
          margin={{
            top: 5,
            right: 30,
            left: 20,
            bottom: 5,
          }}
        >
          <CartesianGrid strokeDasharray="3 3" />
          <XAxis dataKey="name" />
          <YAxis />
          <Tooltip />
          <Line type="monotone" dataKey="uv" stroke="#82ca9d" />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}

export default LinechartChart;
