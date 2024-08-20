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
import { Box, Flex, HStack, Text } from "@chakra-ui/react";

function LinechartChart({ investment_mode = "expert", ...props }) {
  const data = [
    { name: "2018", uv: 1000 },
    { name: "2019", uv: 1700 },
    { name: "2020", uv: 2000 },
    { name: "2021", uv: 2780 },
    { name: "2022", uv: 1890 },
    { name: "2023", uv: 2390 },
    { name: "2024", uv: 3490 },
  ];

  const calculatePerformance = () => {
    const firstValue = data[0].uv;
    const lastValue = data[data.length - 1].uv;
    return (((lastValue - firstValue) / firstValue) * 100).toFixed(2);
  };

  const highestValue = Math.max(...data.map((item) => item.uv));
  const lowestValue = Math.min(...data.map((item) => item.uv));

  const renderAdvancedData = () => (
    <Box
      bg="gray.50"
      py={1}
      px={3}
      borderRadius="md"
      boxShadow="sm"
      border="1px"
      borderColor="gray.200"
      fontSize="xs"
    >
      <HStack spacing={4}>
        <Text fontWeight="semibold">Advanced Data:</Text>
        <Text>Performance: {calculatePerformance()}%</Text>
        <Text>Highest: {highestValue}</Text>
        <Text>Lowest: {lowestValue}</Text>
      </HStack>
    </Box>
  );

  return (
    <Flex direction="column" {...props}>
      {investment_mode === "expert" && (
        <Box alignSelf="flex-end" mb={2}>
          {renderAdvancedData()}
        </Box>
      )}
      <Box width="100%" height="600px">
        <ResponsiveContainer>
          <LineChart
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
            <Line
              type="monotone"
              dataKey="uv"
              stroke="#82ca9d"
              isAnimationActive={investment_mode === "expert"}
            />
          </LineChart>
        </ResponsiveContainer>
      </Box>
    </Flex>
  );
}

export default LinechartChart;
