import React, { useMemo, useState } from "react";
import {
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import {
  Box,
  Flex,
  HStack,
  Select,
  Stat,
  StatArrow,
  StatHelpText,
  StatLabel,
  StatNumber,
  Text,
} from "@chakra-ui/react";

function LinechartChart({ investment_mode = "expert", ...props }) {
  const [scale, setScale] = useState("linear");

  const data = [
    { name: "2018", uv: 1000 },
    { name: "2019", uv: 1700 },
    { name: "2020", uv: 2000 },
    { name: "2021", uv: 2780 },
    { name: "2022", uv: 1890 },
    { name: "2023", uv: 2390 },
    { name: "2024", uv: 3490 },
  ];

  const transformedData = useMemo(() => {
    if (scale === "log") {
      return data.map((item) => ({
        ...item,
        uv: Math.log10(item.uv),
      }));
    }
    return data;
  }, [data, scale]);

  const calculatePerformance = () => {
    const firstValue = data[0].uv;
    const lastValue = data[data.length - 1].uv;
    return (((lastValue - firstValue) / firstValue) * 100).toFixed(2);
  };

  const highestValue = Math.max(...data.map((item) => item.uv));
  const lowestValue = Math.min(...data.map((item) => item.uv));
  const finalPrice = data[data.length - 1].uv;
  const performanceValue = calculatePerformance();

  const renderStat = () => (
    <Stat>
      <StatLabel>Final Price</StatLabel>
      <StatNumber fontSize="lg">${finalPrice.toLocaleString()}</StatNumber>
      <StatHelpText>
        <StatArrow type={performanceValue > 0 ? "increase" : "decrease"} />
        {performanceValue}%
      </StatHelpText>
    </Stat>
  );

  const renderAdvancedData = () => (
    <HStack spacing={4}>
      <Text fontWeight="semibold">Advanced Data:</Text>
      <Text>Highest: ${highestValue.toLocaleString()}</Text>
      <Text>Lowest: ${lowestValue.toLocaleString()}</Text>
      <Select
        size="xs"
        value={scale}
        onChange={(e) => setScale(e.target.value)}
      >
        <option value="linear">Linear</option>
        <option value="log">Logarithmic</option>
      </Select>
    </HStack>
  );

  const formatYAxis = (value) => {
    if (scale === "log") {
      return Math.pow(10, value).toFixed(0);
    }
    return value;
  };

  return (
    <Flex direction="column" {...props}>
      <Flex
        bg="gray.50"
        py={2}
        px={4}
        borderRadius="md"
        boxShadow="sm"
        border="1px"
        borderColor="gray.200"
        fontSize="xs"
        justifyContent="space-between"
        alignItems="center"
        width="100%"
        mb={2}
      >
        {renderStat()}
        {investment_mode === "expert" && renderAdvancedData()}
      </Flex>
      <Box width="100%" height="600px">
        <ResponsiveContainer>
          <LineChart
            data={transformedData}
            margin={{
              top: 5,
              right: 30,
              left: 20,
              bottom: 5,
            }}
          >
            <CartesianGrid strokeDasharray="3 3" />
            <XAxis dataKey="name" />
            <YAxis
              scale={scale === "log" ? "log" : "auto"}
              domain={["auto", "auto"]}
              tickFormatter={formatYAxis}
            />
            <Tooltip
              formatter={(value) =>
                scale === "log" ? Math.pow(10, value).toFixed(0) : value
              }
            />
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
