import {
  Activity,
  ArrowUpRight,
  BarChart3,
  Bitcoin,
  BriefcaseBusiness,
  CircleDollarSign,
  Coins,
  Compass,
  Gem,
  Goal,
  Landmark,
  LineChart,
  PiggyBank,
  PieChart,
  Shield,
  Sparkles,
  Target,
  TrendingUp,
  Wallet,
  Zap,
  type LucideIcon,
  type LucideProps,
} from "lucide-react";
import { cn } from "../../lib/cn";

const portfolioIconEntries = [
  ["activity", "Activity", Activity],
  ["arrow-up-right", "Arrow up", ArrowUpRight],
  ["bar-chart", "Bar chart", BarChart3],
  ["bitcoin", "Bitcoin", Bitcoin],
  ["briefcase", "Briefcase", BriefcaseBusiness],
  ["cash", "Cash", CircleDollarSign],
  ["coins", "Coins", Coins],
  ["compass", "Compass", Compass],
  ["gem", "Gem", Gem],
  ["goal", "Goal", Goal],
  ["landmark", "Landmark", Landmark],
  ["line-chart", "Line chart", LineChart],
  ["piggy-bank", "Piggy bank", PiggyBank],
  ["pie-chart", "Pie chart", PieChart],
  ["shield", "Shield", Shield],
  ["sparkles", "Sparkles", Sparkles],
  ["target", "Target", Target],
  ["trending-up", "Trending up", TrendingUp],
  ["wallet", "Wallet", Wallet],
  ["zap", "Zap", Zap],
] as const;

export type PortfolioIconId = (typeof portfolioIconEntries)[number][0];
export type PortfolioIconSize = number | string;

export const portfolioIconOptions = portfolioIconEntries.map(([id, label]) => ({
  id,
  label,
}));

const portfolioIconRegistry = Object.fromEntries(
  portfolioIconEntries.map(([id, , Icon]) => [id, Icon])
) as unknown as Record<PortfolioIconId, LucideIcon>;

export interface PortfolioIconProps extends Omit<LucideProps, "ref"> {
  id?: PortfolioIconId;
  size?: PortfolioIconSize;
}

/** Renders one of the finite, product-approved portfolio identity icons. */
export function PortfolioIcon({
  id = "pie-chart",
  size = 24,
  className,
  ...props
}: PortfolioIconProps) {
  const Icon = portfolioIconRegistry[id] ?? portfolioIconRegistry["pie-chart"];
  return (
    <Icon
      aria-hidden
      className={cn("ds-portfolio-icon", className)}
      size={size}
      {...props}
    />
  );
}
