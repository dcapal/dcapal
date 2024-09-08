import React, { useEffect, useState } from "react";
import { DCAPAL_API } from "@app/config";
import { api } from "@app/api";
import { ContainerPage } from "./containerPage";
import { useNavigate } from "react-router-dom";
import {
  Button,
  FormControl,
  FormLabel,
  Input,
  Menu,
  MenuButton,
  MenuItem,
  MenuList,
  NumberDecrementStepper,
  NumberIncrementStepper,
  NumberInput,
  NumberInputField,
  NumberInputStepper,
  Select,
  Switch,
  Text,
  Tooltip,
  useToast,
} from "@chakra-ui/react";
import { ChevronDownIcon } from "@chakra-ui/icons";

export default function InvestmentSettings({ session, editMode = false }) {
  const [investmentSettingsData, setInvestmentSettingsData] = useState(null);
  const [isEditing, setIsEditing] = useState(editMode);
  const toast = useToast();
  const navigate = useNavigate();

  const handleMenuItemClick = (path) => {
    navigate(path);
  };

  const config = {
    headers: { Authorization: `Bearer ${session.access_token}` },
  };

  const fetchProfile = async () => {
    try {
      const response = await api.get(
        `${DCAPAL_API}/v1/user/investment-preferences`,
        config
      );
      setUserData({
        riskTolerance: response.data.riskTolerance,
        investmentHorizon: response.data.investmentHorizon,
        investmentMode: response.data.investmentMode,
        investmentGoal: response.data.investmentGoal,
        aiEnabled: response.data.aiEnabled,
      });
    } catch (error) {
      console.error("Error fetching data:", error);
    }
  };

  const [userData, setUserData] = useState({
    riskTolerance: "Low",
    investmentHorizon: 1,
    investmentMode: "Standard",
    investmentGoal: "Retirement",
    aiEnabled: false,
  });

  const handleInputChange = (nameOrEvent, value) => {
    let inputName, inputValue;

    if (typeof nameOrEvent === "string") {
      inputName = nameOrEvent;
      inputValue = value;
    } else if (nameOrEvent && nameOrEvent.target) {
      inputName = nameOrEvent.target.name;
      inputValue = nameOrEvent.target.value;
    } else {
      console.error("Unexpected input type");
      return;
    }

    // Special handling for the aiEnabled switch
    if (inputName === "aiEnabled") {
      inputValue = !userData.aiEnabled; // Toggle the current value
    }

    setUserData((prevData) => ({
      ...prevData,
      [inputName]: inputValue,
    }));

    console.log(`Updated ${inputName} to:`, inputValue); // Add this for debugging
  };
  useEffect(() => {
    fetchProfile();
  }, []);

  const handleSave = async () => {
    try {
      const dataToSend = {
        riskTolerance: userData.riskTolerance,
        investmentHorizon: userData.investmentHorizon,
        investmentMode: userData.investmentMode,
        investmentGoal: userData.investmentGoal,
        aiEnabled: userData.aiEnabled,
      };

      await api.post(
        `${DCAPAL_API}/v1/user/investment-preferences`,
        dataToSend,
        config
      );

      console.log("edit mode:", editMode); // Add this for debugging
      if (editMode) {
        navigate("/investment-settings");
      } else {
        setIsEditing(false);
        fetchProfile(); // Refresh the data after updating
      }

      toast({
        title: "Profile updated",
        status: "success",
        duration: 2000,
        isClosable: true,
      });
    } catch (error) {
      toast({
        title: "Error updating profile",
        description: error.message,
        status: "error",
        duration: 2000,
        isClosable: true,
      });
    }
  };
  return (
    <ContainerPage
      title={"Investment Settings"}
      content={
        <div className="w-full flex flex-col grow justify-center items-center  gap-8 bg-gray-100">
          {editMode && (
            <div className="w-full max-w-4xl p-4 bg-white rounded-lg shadow-md mb-4">
              Welcome to DcaPal! To Personalize your investment experience,
              please provide some information about your investment preferences
              and goals. You can always change these settings later.
            </div>
          )}
          <div className="w-full max-w-4xl p-4 bg-white rounded-lg shadow-md">
            {!editMode && (
              <div className="flex items-center justify-between p-4 border-b">
                <Menu>
                  <MenuButton
                    as="div"
                    className="flex items-center cursor-pointer large-text"
                    fontWeight="semibold"
                  >
                    Investment Settings
                    <ChevronDownIcon ml={2} />
                  </MenuButton>
                  <MenuList>
                    <MenuItem onClick={() => handleMenuItemClick("/profile")}>
                      Profile
                    </MenuItem>
                    <MenuItem
                      onClick={() =>
                        handleMenuItemClick("/investment-settings")
                      }
                    >
                      Investment Settings
                    </MenuItem>
                  </MenuList>
                </Menu>
              </div>
            )}
            <div className="p-4 space-y-6">
              <div className="flex items-center space-x-4">
                <Tooltip label="Choose between Standard mode for guided investing or Expert mode for more control">
                  <label className="w-1/4 text-lg font-semibold">
                    Investment Mode
                  </label>
                </Tooltip>
                {isEditing ? (
                  <Select
                    name="investmentMode"
                    value={userData.investmentMode}
                    className="w-3/4"
                    isReadOnly={!isEditing}
                    onChange={handleInputChange}
                  >
                    <option value="Standard">Standard</option>
                    <option value="Expert">Expert</option>
                  </Select>
                ) : (
                  <Input
                    value={userData.investmentMode}
                    className="w-3/4"
                    readOnly={true}
                  />
                )}
              </div>
              <div className="flex items-center space-x-4">
                <Tooltip label="Select the duration over which you expect to achieve your financial or investment goals. This helps to assess the level of risk and suitable strategies for your plan.">
                  <label className="w-1/4 text-lg font-semibold">
                    Time Horizon
                  </label>
                </Tooltip>
                {isEditing ? (
                  <NumberInput
                    name="investmentHorizon"
                    value={userData.investmentHorizon}
                    min={1}
                    max={20}
                    className="w-full"
                    isReadOnly={!isEditing}
                    onChange={(valueString, valueNumber) =>
                      handleInputChange("investmentHorizon", valueNumber)
                    }
                  >
                    <NumberInputField />
                    <NumberInputStepper>
                      <NumberIncrementStepper />
                      <NumberDecrementStepper />
                    </NumberInputStepper>
                  </NumberInput>
                ) : (
                  <Input
                    value={userData.investmentHorizon}
                    className="w-3/4"
                    readOnly={true}
                  />
                )}
              </div>
              <div className="flex items-center space-x-4">
                <Tooltip label="Indicate the level of risk you are comfortable taking with your investments. This helps to tailor strategies to your risk preferences, whether conservative, moderate, or aggressive.">
                  <label className="w-1/4 text-lg font-semibold">
                    Risk Tolerance
                  </label>
                </Tooltip>
                {isEditing ? (
                  <Select
                    name="riskTolerance"
                    value={userData.riskTolerance}
                    className="w-3/4"
                    isReadOnly={!isEditing}
                    onChange={handleInputChange}
                  >
                    <option value="Low">Low</option>
                    <option value="Medium">Medium</option>
                    <option value="High">High</option>
                  </Select>
                ) : (
                  <Input
                    value={userData.riskTolerance}
                    className="w-3/4"
                    readOnly={true}
                  />
                )}
              </div>
              <div className="flex items-center space-x-4">
                <Tooltip label="Specify the primary objective of your investment, such as capital Retirement, Education, Wealth Building or Other. This helps to align your strategy with your financial objectives.">
                  <label className="w-1/4 text-lg font-semibold">
                    Investment Goal
                  </label>
                </Tooltip>
                {isEditing ? (
                  <Select
                    name="investmentGoal"
                    value={userData.investmentGoal}
                    className="w-3/4"
                    isReadOnly={!isEditing}
                    onChange={handleInputChange}
                  >
                    <option value="Retirement">Retirement</option>
                    <option value="Education">Education</option>
                    <option value="Wealth Building">Wealth Building</option>
                    <option value="Other">Other</option>
                  </Select>
                ) : (
                  <Input
                    value={userData.investmentGoal}
                    className="w-3/4"
                    readOnly={true}
                  />
                )}
              </div>
              <div className="flex items-center space-x-4">
                <Tooltip label="Enable this option to allow AI-powered tools to assist in optimizing your financial strategies and decision-making processes.">
                  <label className="w-1/4 text-lg font-semibold">AI*</label>
                </Tooltip>
                <FormControl display="flex" alignItems="center">
                  <FormLabel htmlFor="aiEnabled" mb="0">
                    Enable
                  </FormLabel>
                  <Switch
                    name="aiEnabled"
                    isDisabled={!isEditing}
                    isChecked={userData.aiEnabled}
                    onChange={() => handleInputChange("aiEnabled")}
                  />
                </FormControl>
              </div>
            </div>
            <div className="flex justify-end p-4 border-t">
              {isEditing ? (
                <Button onClick={handleSave}>Confirm</Button>
              ) : (
                <Button onClick={() => setIsEditing(true)}>Edit</Button>
              )}
            </div>
            <Text fontSize="md" ml={5}>
              *Requires Personal Data Usage and Sending data to OpenAI
            </Text>
          </div>
        </div>
      }
    />
  );
}
