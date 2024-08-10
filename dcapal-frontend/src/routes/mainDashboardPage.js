import React, { useEffect, useState } from "react";
import { DCAPAL_API, supabase } from "@app/config";
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
  Line,
  LineChart,
  Pie,
  PieChart,
  ResponsiveContainer,
  XAxis,
} from "recharts";

export default function Dashboard({ session }) {
  const data = [
    {
      name: "Page A",
      uv: 4000,
      pv: 2400,
      amt: 2400,
    },
    {
      name: "Page B",
      uv: 3000,
      pv: 1398,
      amt: 2210,
    },
    {
      name: "Page C",
      uv: 2000,
      pv: 9800,
      amt: 2290,
    },
    {
      name: "Page D",
      uv: 2780,
      pv: 3908,
      amt: 2000,
    },
    {
      name: "Page E",
      uv: 1890,
      pv: 4800,
      amt: 2181,
    },
    {
      name: "Page F",
      uv: 2390,
      pv: 3800,
      amt: 2500,
    },
    {
      name: "Page G",
      uv: 3490,
      pv: 4300,
      amt: 2100,
    },
  ];

  const [loading, setLoading] = useState(true);
  const [username, setUsername] = useState(null);
  const [website, setWebsite] = useState(null);
  const [avatar_url, setAvatarUrl] = useState(null);

  const config = {
    headers: { Authorization: `Bearer ${session.access_token}` },
  };

  useEffect(() => {
    let ignore = false;

    async function getProfile() {
      setLoading(true);
      const { user } = session;

      const { data, error } = await api.get(`${DCAPAL_API}/protected`, config);

      if (!ignore) {
        if (error) {
          console.warn(error);
        } else if (data) {
          //setUsername(data.username);
          //setWebsite(data.website);
          //setAvatarUrl(data.avatar_url);
          setUsername(data?.data?.user?.email);
        }
      }

      setLoading(false);
    }

    getProfile();

    return () => {
      ignore = true;
    };
  }, [session]);

  const [todos, setTodos] = useState([]);

  async function updateProfile(event, avatarUrl) {
    event.preventDefault();

    setLoading(true);
    const { user } = session;

    const updates = {
      id: user.id,
      username,
      website,
      avatar_url: avatarUrl,
      updated_at: new Date(),
    };

    const { error } = await supabase.from("profiles").upsert(updates);

    if (error) {
      alert(error.message);
    } else {
      setAvatarUrl(avatarUrl);
    }
    setLoading(false);
  }

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
                    <Table>
                      <Thead>
                        <Tr>
                          <Th>Data1</Th>
                          <Th>Data2</Th>
                          <Th>Data2</Th>
                          <Th>Data2</Th>
                          <Th>Data2</Th>
                          <Th>Data2</Th>
                        </Tr>
                      </Thead>
                      <Tbody>
                        <Tr>
                          <Td>12</Td>
                          <Td>45.67</Td>
                          <Td>-23.45</Td>
                          <Td>78.90</Td>
                          <Td>-12.34</Td>
                          <Td>56.78</Td>
                        </Tr>
                        <Tr>
                          <Td>34</Td>
                          <Td>-67.89</Td>
                          <Td>23.45</Td>
                          <Td>-89.01</Td>
                          <Td>45.67</Td>
                          <Td>-12.34</Td>
                        </Tr>
                        <Tr>
                          <Td>56</Td>
                          <Td>12.34</Td>
                          <Td>-45.67</Td>
                          <Td>89.01</Td>
                          <Td>-23.45</Td>
                          <Td>67.89</Td>
                        </Tr>
                        <Tr>
                          <Td>78</Td>
                          <Td>-34.56</Td>
                          <Td>56.78</Td>
                          <Td>-12.34</Td>
                          <Td>78.90</Td>
                          <Td>-45.67</Td>
                        </Tr>
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
      <ResponsiveContainer
        config={{
          desktop: {
            label: "Desktop",
            color: "hsl(var(--chart-1))",
          },
        }}
        className="min-h-[300px]"
      >
        <BarChart
          accessibilityLayer
          data={[
            { month: "January", desktop: 186 },
            { month: "February", desktop: 305 },
            { month: "March", desktop: 237 },
            { month: "April", desktop: 73 },
            { month: "May", desktop: 209 },
            { month: "June", desktop: 214 },
          ]}
        >
          <CartesianGrid vertical={false} />
          <XAxis
            dataKey="month"
            tickLine={false}
            tickMargin={10}
            axisLine={false}
            tickFormatter={(value) => value.slice(0, 3)}
          />
          <Bar dataKey="desktop" fill="var(--color-desktop)" radius={8} />
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
  return (
    <div {...props}>
      <ResponsiveContainer
        config={{
          visitors: {
            label: "Visitors",
          },
          chrome: {
            label: "Chrome",
            color: "hsl(var(--chart-1))",
          },
          safari: {
            label: "Safari",
            color: "hsl(var(--chart-2))",
          },
          firefox: {
            label: "Firefox",
            color: "hsl(var(--chart-3))",
          },
          edge: {
            label: "Edge",
            color: "hsl(var(--chart-4))",
          },
          other: {
            label: "Other",
            color: "hsl(var(--chart-5))",
          },
        }}
      >
        <PieChart>
          <Pie
            data={[
              { browser: "chrome", visitors: 275, fill: "var(--color-chrome)" },
              { browser: "safari", visitors: 200, fill: "var(--color-safari)" },
              {
                browser: "firefox",
                visitors: 187,
                fill: "var(--color-firefox)",
              },
              { browser: "edge", visitors: 173, fill: "var(--color-edge)" },
              { browser: "other", visitors: 90, fill: "var(--color-other)" },
            ]}
            dataKey="visitors"
            nameKey="browser"
          />
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
