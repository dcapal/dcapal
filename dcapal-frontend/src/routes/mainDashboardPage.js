import React, { useEffect, useState } from "react";
import { DCAPAL_API } from "@app/config";
import { api } from "@app/api";
import {
  Button,
  Menu,
  MenuButton,
  MenuItem,
  MenuList,
  Table,
  TableContainer,
  Tbody,
  Td,
  Th,
  Thead,
  Tr,
} from "@chakra-ui/react";
import { ContainerPage } from "./containerPage";
import { ChevronDownIcon } from "@chakra-ui/icons";

import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Legend,
  Line,
  LineChart,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

export default function Dashboard({ session }) {
  const mockData = [
    { name: "Asset A", weight: 25.5 },
    { name: "Asset B", weight: 30.2 },
    { name: "Asset C", weight: 15.8 },
    { name: "Asset D", weight: 28.5 },
  ];
  const [holdings, setHoldings] = useState([]);

  const [loading, setLoading] = useState(true);
  const [username, setUsername] = useState(null);
  const [website, setWebsite] = useState(null);
  const [avatar_url, setAvatarUrl] = useState(null);

  const config = {
    headers: { Authorization: `Bearer ${session.access_token}` },
  };

  useEffect(() => {
    fetchData();
  }, []);

  const fetchData = async () => {
    try {
      const response = await api.get(
        `${DCAPAL_API}/v1/user/portfolios/uuid_v4/holdings`,
        config
      );
      setHoldings(response.data.holdings);
    } catch (error) {
      console.error("Error fetching data:", error);
    }
  };

  // Get table headers dynamically from the first holding object
  const getHeaders = () => {
    if (holdings.length === 0) return [];
    return Object.keys(holdings[0]);
  };

  return (
    <ContainerPage
      title={"Dashboard"}
      content={
        // w-full flex flex-col grow justify-center items-center text-center gap-8 bg-gray-100
        <div className="w-screen grow">
          <header className="bg-background shadow-sm sticky top-0 z-10 px-4 py-3 flex items-center justify-between sm:px-6">
            <div className="flex items-center gap-4">
              <Menu>
                <MenuButton as={Button} rightIcon={<ChevronDownIcon />}>
                  Menu
                </MenuButton>
                <MenuList align="start">
                  <MenuItem>Dashboard</MenuItem>
                  <MenuItem>Analytics</MenuItem>
                  <MenuItem>Settings</MenuItem>
                </MenuList>
              </Menu>
              <Menu>
                <MenuButton as={Button} rightIcon={<ChevronDownIcon />}>
                  View
                </MenuButton>
                <MenuList align="start">
                  <MenuItem>List</MenuItem>
                  <MenuItem>Grid</MenuItem>
                  <MenuItem>Calendar</MenuItem>
                </MenuList>
              </Menu>
            </div>
            <Button size="sm">
              <PlusIcon className="h-4 w-4 mr-2" />
              Add New Portfolio
            </Button>
          </header>
          <div className="flex-1 grid grid-cols-1 gap-4 p-4 sm:grid-cols-2 sm:gap-6 md:grid-cols-2 md:p-6 lg:gap-8">
            <div className="bg-background rounded-lg shadow-lg flex flex-col">
              <div className="p-4 sm:p-6 flex-1">
                <div className="flex items-center justify-between">
                  <h3 className="text-lg font-medium">Pie Chart</h3>
                  <div className="flex items-center gap-2 hidden sm:flex">
                    <span className="text-sm text-muted-foreground">
                      Legend:
                    </span>
                    <div className="flex items-center gap-2">
                      <div className="w-3 h-3 rounded-full bg-primary" />
                      <span className="text-sm">1</span>
                      <div className="w-3 h-3 rounded-full bg-secondary" />
                      <span className="text-sm">2</span>
                      <div className="w-3 h-3 rounded-full bg-accent" />
                      <span className="text-sm">3</span>
                      <div className="w-3 h-3 rounded-full bg-muted" />
                      <span className="text-sm">4</span>
                    </div>
                  </div>
                </div>
                <PiechartcustomChart className="aspect-[9/4] w-full" />
              </div>
            </div>
            <div className="bg-background rounded-lg shadow-lg flex flex-col">
              <div className="p-4 sm:p-6 flex-1">
                <h3 className="text-lg font-medium">Bar Chart</h3>
                <BarchartChart className="aspect-[9/4] w-full" />
              </div>
            </div>
            <div className="bg-background rounded-lg shadow-lg col-span-1 sm:col-span-2">
              <div className="p-4 sm:p-6">
                <div className="flex items-center justify-between">
                  <h3 className="text-lg font-medium">List1</h3>
                  <Button variant="ghost" size="icon">
                    <ExpandIcon className="h-5 w-5" />
                    <span className="sr-only">More</span>
                  </Button>
                </div>
                <div className="overflow-x-auto">
                  <TableContainer>
                    <Table variant="simple">
                      <Thead>
                        <Tr>
                          {getHeaders().map((header, index) => (
                            <Th key={index}>
                              {header.replace("_", " ").toUpperCase()}
                            </Th>
                          ))}
                        </Tr>
                      </Thead>
                      <Tbody>
                        {holdings.map((holding, rowIndex) => (
                          <Tr key={rowIndex}>
                            {getHeaders().map((header, cellIndex) => (
                              <Td key={cellIndex}>
                                {typeof holding[header] === "number"
                                  ? holding[header].toFixed(2)
                                  : holding[header]}
                              </Td>
                            ))}
                          </Tr>
                        ))}
                      </Tbody>
                    </Table>
                  </TableContainer>
                </div>
              </div>
            </div>
          </div>
        </div>
      }
    />
  );
}

function BarchartChart(props) {
  return (
    <div {...props}>
      <ResponsiveContainer width="100%" height={400}>
        <BarChart
          data={[
            { year: 2020, gain: 15.2 },
            { year: 2021, gain: 22.8 },
            { year: 2022, gain: -5.1 },
            { year: 2023, gain: 18.3 },
            { year: 2024, gain: 10.5 },
          ]}
          margin={{
            top: 20,
            right: 30,
            left: 20,
            bottom: 5,
          }}
        >
          <CartesianGrid strokeDasharray="3 3" />
          <XAxis dataKey="year" />
          <YAxis
            yAxisId="left"
            orientation="left"
            stroke="#8884d8"
            label={{
              value: "Performance (%)",
              angle: -90,
              position: "insideLeft",
            }}
          />
          <Tooltip />
          <Legend />
          <Bar
            yAxisId="left"
            dataKey="gain"
            fill="#8884d8"
            name="Annual Gain (%)"
          />
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}

function ExpandIcon(props) {
  return (
    <svg
      {...props}
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="m21 21-6-6m6 6v-4.8m0 4.8h-4.8" />
      <path d="M3 16.2V21m0 0h4.8M3 21l6-6" />
      <path d="M21 7.8V3m0 0h-4.8M21 3l-6 6" />
      <path d="M3 7.8V3m0 0h4.8M3 3l6 6" />
    </svg>
  );
}

function LinechartChart(props) {
  return (
    <div {...props}>
      <ResponsiveContainer
        config={{
          desktop: {
            label: "Desktop",
            color: "hsl(var(--chart-1))",
          },
        }}
      >
        <LineChart
          accessibilityLayer
          data={[
            { month: "January", desktop: 186 },
            { month: "February", desktop: 305 },
            { month: "March", desktop: 237 },
            { month: "April", desktop: 73 },
            { month: "May", desktop: 209 },
            { month: "June", desktop: 214 },
          ]}
          margin={{
            left: 12,
            right: 12,
          }}
        >
          <CartesianGrid vertical={false} />
          <XAxis
            dataKey="month"
            tickLine={false}
            axisLine={false}
            tickMargin={8}
            tickFormatter={(value) => value.slice(0, 3)}
          />

          <Line
            dataKey="desktop"
            type="natural"
            stroke="var(--color-desktop)"
            strokeWidth={2}
            dot={false}
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}

function MenuIcon(props) {
  return (
    <svg
      {...props}
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <line x1="4" x2="20" y1="12" y2="12" />
      <line x1="4" x2="20" y1="6" y2="6" />
      <line x1="4" x2="20" y1="18" y2="18" />
    </svg>
  );
}

function PiechartcustomChart(props) {
  const mockData = [
    { name: "Asset A", weight: 25.5 },
    { name: "Asset B", weight: 30.2 },
    { name: "Asset C", weight: 15.8 },
    { name: "Asset D", weight: 28.5 },
  ];
  const COLORS = ["#0088FE", "#00C49F", "#FFBB28", "#FF8042", "#8884D8"];
  return (
    <div {...props}>
      <ResponsiveContainer width="100%" height={400}>
        <PieChart>
          <Pie
            data={mockData}
            dataKey="weight"
            nameKey="name"
            cx="50%"
            cy="50%"
            outerRadius={150}
            fill="#8884d8"
            label={(entry) => `${entry.name}: ${entry.weight.toFixed(2)}%`}
          >
            {mockData.map((entry, index) => (
              <Cell
                key={`cell-${index}`}
                fill={COLORS[index % COLORS.length]}
              />
            ))}
          </Pie>
          <Tooltip formatter={(value) => `${value.toFixed(2)}%`} />
          <Legend />
        </PieChart>
      </ResponsiveContainer>
    </div>
  );
}

function PlusIcon(props) {
  return (
    <svg
      {...props}
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M5 12h14" />
      <path d="M12 5v14" />
    </svg>
  );
}

function ViewIcon(props) {
  return (
    <svg
      {...props}
      xmlns="http://www.w3.org/2000/svg"
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M5 12s2.545-5 7-5c4.454 0 7 5 7 5s-2.546 5-7 5c-4.455 0-7-5-7-5z" />
      <path d="M12 13a1 1 0 1 0 0-2 1 1 0 0 0 0 2z" />
      <path d="M21 17v2a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-2" />
      <path d="M21 7V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v2" />
    </svg>
  );
}
