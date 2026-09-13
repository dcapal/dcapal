import type { Meta, StoryObj } from "@storybook/react";
import {
  Alert,
  AssetTickerChip,
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Checkbox,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
  EmptyState,
  Field,
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
  Input,
  InputWithSuffix as InputWithSuffixControl,
  Skeleton,
  SegmentedControl,
  RadioCardGroup,
  Switch,
  Textarea,
} from "../../components/ui";

const meta = {
  title: "UI / Primitives",
  component: Button,
  tags: ["autodocs"],
  parameters: { layout: "padded" },
} satisfies Meta<typeof Button>;
export default meta;
type Story = StoryObj<typeof meta>;

export const ButtonStates: Story = {
  render: () => (
    <div className="dcapal-theme ds-story-row">
      <Button>Primary action</Button>
      <Button variant="outline">Outline</Button>
      <Button variant="secondary">Secondary</Button>
      <Button variant="ghost">Ghost</Button>
      <Button variant="destructive">Remove</Button>
      <Button disabled>Disabled</Button>
      <Button aria-busy>Saving…</Button>
    </div>
  ),
};

export const ButtonPressedStates: Story = {
  render: () => (
    <div className="dcapal-theme ds-story-row">
      <Button pressed>Primary pressed</Button>
      <Button pressed variant="secondary">
        Secondary pressed
      </Button>
      <Button pressed variant="outline">
        Outline pressed
      </Button>
      <Button pressed variant="ghost">
        Ghost pressed
      </Button>
      <Button pressed variant="destructive">
        Destructive pressed
      </Button>
      <Button pressed variant="link">
        Link pressed
      </Button>
    </div>
  ),
  tags: ["!autodocs"],
};

export const FormStates: Story = {
  render: () => (
    <div
      className="dcapal-theme"
      style={{ maxWidth: "var(--dcapal-form-width)" }}
    >
      <FieldGroup>
        <Field>
          <FieldLabel htmlFor="asset-name">Asset name</FieldLabel>
          <Input id="asset-name" placeholder="Search by name or ticker" />
          <FieldDescription>
            Use an exchange ticker when possible.
          </FieldDescription>
        </Field>
        <Field invalid>
          <FieldLabel htmlFor="target-weight">Target weight</FieldLabel>
          <Input id="target-weight" aria-invalid placeholder="0" />
          <FieldError match>Enter a target between 0% and 100%.</FieldError>
        </Field>
        <Field>
          <FieldLabel htmlFor="notes">Notes</FieldLabel>
          <Textarea id="notes" placeholder="Optional context" />
        </Field>
        <label className="ds-story-row">
          <Checkbox aria-label="Use fractional quantities" /> Use fractional
          quantities
        </label>
        <label className="ds-story-row">
          <Switch aria-label="Enable alerts" /> Enable alerts
        </label>
      </FieldGroup>
    </div>
  ),
};

export const InputWithSuffix: Story = {
  render: () => (
    <div
      className="dcapal-theme"
      style={{ maxWidth: "var(--dcapal-form-width)" }}
    >
      <Field>
        <FieldLabel htmlFor="budget">Investment budget</FieldLabel>
        <InputWithSuffixControl
          id="budget"
          defaultValue="1,000"
          inputMode="decimal"
          suffix="EUR"
        />
      </Field>
    </div>
  ),
  tags: ["!autodocs"],
};

export const SegmentedControls: Story = {
  render: () => (
    <div className="dcapal-theme">
      <SegmentedControl
        defaultValue="fixed"
        label="Transaction fee policy"
        size="sm"
        options={[
          { value: "zero", label: "Zero" },
          { value: "fixed", label: "Fixed" },
          { value: "variable", label: "Variable" },
        ]}
      />
    </div>
  ),
  tags: ["!autodocs"],
};

export const RadioCardAllocationMethod: Story = {
  render: () => (
    <div className="dcapal-theme" style={{ maxWidth: "28rem" }}>
      <RadioCardGroup
        label="Allocation method"
        defaultValue="simple"
        options={[
          {
            value: "simple",
            title: "Simple",
            description: "Use absolute target weights for each asset.",
          },
          {
            value: "strategic",
            title: "Strategic",
            description:
              "Use asset-class targets with relative weights inside each class.",
          },
        ]}
      />
    </div>
  ),
  tags: ["!autodocs"],
};

export const TickerChipsAndAverageCost: Story = {
  render: () => (
    <div className="dcapal-theme ds-story-row">
      <AssetTickerChip ticker="VWCE" />
      <span className="ds-asset-card__stat">
        <span className="ds-asset-card__stat-label">Average cost basis</span>
        <span className="ds-asset-card__stat-value">€97.37</span>
      </span>
      <span className="ds-asset-card__stat">
        <span className="ds-asset-card__stat-label">Average cost basis</span>
        <span className="ds-asset-card__stat-value">—</span>
      </span>
    </div>
  ),
  tags: ["!autodocs"],
};

export const DisabledSwitch: Story = {
  render: () => (
    <div className="dcapal-theme ds-story-row">
      <Switch disabled aria-label="Enable alerts" />
      <span>Alerts unavailable</span>
    </div>
  ),
  tags: ["!autodocs"],
};

export const Feedback: Story = {
  render: () => (
    <div className="dcapal-theme ds-story-stack">
      <Alert variant="success">
        <div>
          <strong>Allocation applied</strong>
          <p>Portfolio weights were updated.</p>
        </div>
      </Alert>
      <Alert variant="warning">
        <div>
          <strong>Drift above target</strong>
          <p>Review the allocation details.</p>
        </div>
      </Alert>
      <Alert variant="destructive">
        <div>
          <strong>Could not save changes</strong>
          <p>Try again or check the connection.</p>
        </div>
      </Alert>
      <div className="ds-story-row">
        <Badge variant="success">Within target</Badge>
        <Badge variant="warning">+4.8 pp</Badge>
        <Badge variant="destructive">Error</Badge>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>Loading state</CardTitle>
          <CardDescription>
            Use Skeleton for content placeholders.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Skeleton style={{ width: "70%" }} />
        </CardContent>
      </Card>
      <EmptyState title="No results" description="Try another search term." />
    </div>
  ),
};

export const DialogStates: Story = {
  render: () => (
    <div className="dcapal-theme ds-story-row">
      <Dialog defaultOpen>
        <DialogTrigger render={<Button variant="outline" />}>
          Open dialog
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Confirm allocation</DialogTitle>
            <DialogDescription>
              Escape closes this dialog and restores focus to its trigger.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <DialogClose render={<Button variant="outline" />}>
              Cancel
            </DialogClose>
            <Button>Continue</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  ),
  tags: ["!autodocs"],
};
