import React, { useEffect, useRef, useState } from "react";
import { DCAPAL_API } from "@app/config";
import { api } from "@app/api";
import {
  Button,
  Menu,
  MenuButton,
  MenuItem,
  MenuList,
  Tooltip as ChakraToolTip,
} from "@chakra-ui/react";

import { ContainerPage } from "./containerPage";
import { ChatCard } from "@components/core/aiChatCard";
import { ChevronDownIcon, QuestionOutlineIcon } from "@chakra-ui/icons";
import LinechartChart from "@components/charts/LinechartChart";

import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Legend,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { useNavigate } from "react-router-dom";

export default function HistoricalView({ session }) {
  const mockData = [
    { name: "BTC", weight: 25.5 },
    { name: "ETH", weight: 30.2 },
    { name: "ADA", weight: 15.8 },
    { name: "ALLWD", weight: 28.5 },
  ];
  const [holdings, setHoldings] = useState([]);
  const MAX_VISIBLE_ITEMS = 4;

  const [loading, setLoading] = useState(true);
  const [username, setUsername] = useState(null);
  const [website, setWebsite] = useState(null);
  const [avatar_url, setAvatarUrl] = useState(null);
  const [isChatVisible, setIsChatVisible] = useState(false);
  const [showScroll, setShowScroll] = useState(false);
  const tableRef = useRef(null);
  const [investmentMode, setInvestmentMode] = useState("Standard");
  const navigate = useNavigate();
  const handleMenuItemClick = (path) => {
    navigate(path);
  };

  const fetchInvestmentMode = async () => {
    try {
      const response = await api.get(
        `${DCAPAL_API}/v1/user/investment-preferences`,
        config
      );
      setInvestmentMode(response.data.investmentMode || "Standard");
      console.log("Investment mode:", response.data.investmentMode);
    } catch (error) {
      console.error("Error fetching investment mode:", error);
    }
  };

  useEffect(() => {
    setShowScroll(holdings.length > MAX_VISIBLE_ITEMS);
  }, [holdings]);

  const tableStyle = {
    maxHeight: showScroll ? `${41 + MAX_VISIBLE_ITEMS * 53}px` : "auto",
    overflowY: showScroll ? "auto" : "visible",
  };

  const handleShowChat = () => {
    setIsChatVisible(!isChatVisible);
  };

  const config = {
    headers: { Authorization: `Bearer ${session.access_token}` },
  };

  useEffect(() => {
    fetchData();
    fetchInvestmentMode();
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
        <div className="w-screen grow bg-gray-100">
          <header className="bg-background bg-white shadow-sm sticky top-0 z-10 px-4 py-3 flex items-center justify-between sm:px-6">
            <div className="flex items-center gap-4">
              <Menu>
                <MenuButton as={Button} rightIcon={<ChevronDownIcon />}>
                  Portfolio
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
                  <MenuItem onClick={() => handleMenuItemClick("/dashboard")}>
                    Main Dashboard
                  </MenuItem>
                  <MenuItem onClick={() => handleMenuItemClick("/historical")}>
                    Historical Value View
                  </MenuItem>
                </MenuList>
              </Menu>
            </div>
            <div className="flex items-center gap-2">
              <Button size="sm">
                <PlusIcon className="h-4 w-4 mr-2" />
                Add New Portfolio
              </Button>
              <Button size="sm" onClick={handleShowChat}>
                Ask AI
              </Button>
            </div>
          </header>
          <div
            className={`grid gap-2 p-2 sm:gap-6 md:p-6 lg:gap-8 w-full ${
              isChatVisible ? "grid-cols-1 lg:grid-cols-3" : "grid-cols-1"
            }`}
          >
            <div
              className={`${isChatVisible ? "col-span-1 lg:col-span-2" : "col-span-1"}`}
            >
              <div className="flex items-center">
                <h3 className="text-lg font-medium">
                  Model Portfolio Value (by Year)
                </h3>
                {investmentMode === "Standard" && (
                  <ChakraToolTip
                    label="This graph shows the variation in the portfolio value over a period of time. The line represents the values, which generally increase with some ups and downs. Higher points indicate peaks, while lower points show declines."
                    fontSize="md"
                  >
                    <span className="ml-2">
                      <QuestionOutlineIcon />
                    </span>
                  </ChakraToolTip>
                )}
              </div>
              <div className="bg-background bg-white rounded-lg shadow-lg flex flex-col">
                <div className="p-1 sm:p-2 flex-1">
                  <LinechartChart
                    className=" w-full"
                    investment_mode={investmentMode}
                  />
                </div>
              </div>
            </div>
            {isChatVisible && (
              <div className="col-span-1 flex flex-col">
                <h3 className="text-lg font-medium ">AI Assistant</h3>
                <ChatCard />
              </div>
            )}
          </div>
        </div>
      }
    />
  );
}

function BarchartChart(props) {
  const data = [
    { year: 2019, gain: -20.2 },
    { year: 2020, gain: 15.2 },
    { year: 2021, gain: 22.8 },
    { year: 2022, gain: -5.1 },
    { year: 2023, gain: 18.3 },
    { year: 2024, gain: 10.5 },
  ];

  return (
    <div {...props}>
      <ResponsiveContainer width="100%" height={400}>
        <BarChart
          data={data}
          margin={{
            top: 20,
            right: 30,
            left: 20,
            bottom: 5,
          }}
        >
          <CartesianGrid strokeDasharray="3 3" />
          <XAxis dataKey="year" />
          <YAxis yAxisId="left" orientation="left" />
          <Tooltip />
          {/* <Legend /> */}
          <Bar yAxisId="left" dataKey="gain" name="Annual Gain (%)">
            {data.map((entry, index) => (
              <Cell
                key={`cell-${index}`}
                fill={entry.gain > 0 ? "#68D391" : "#FC8181"}
              />
            ))}
          </Bar>
        </BarChart>
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
    { name: "BTC", weight: 30 },
    { name: "ETH", weight: 30 },
    { name: "ADA", weight: 20 },
    { name: "ALLWD", weight: 10 },
  ];
  const COLORS = ["#63B3ED", "#4FD1C5", "#FFBB28", "#F6E05E", "#F6AD55"];
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
            // label={(entry) => `${entry.name}: ${entry.weight.toFixed(2)}%`}
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
