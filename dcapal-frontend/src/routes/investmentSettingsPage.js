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
  useToast,
} from "@chakra-ui/react";
import { ChevronDownIcon } from "@chakra-ui/icons";

export default function InvestmentSettings({ session }) {
  const [investmentSettingsData, setInvestmentSettingsData] = useState(null);
  const [isEditing, setIsEditing] = useState(false);
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
        risk_tolerance: response.data.risk_tolerance,
        investment_horizon: response.data.investment_horizon,
        investment_mode: response.data.investment_mode,
        investment_goal: response.data.investment_goal,
        ai_enabled: response.data.ai_enabled,
      });
    } catch (error) {
      console.error("Error fetching data:", error);
    }
  };

  const [userData, setUserData] = useState({
    risk_tolerance: "",
    investment_horizon: "",
    investment_mode: "",
    investment_goal: "",
    ai_enabled: false,
  });

  const handleInputChange = (e) => {
    const { name, value } = e.target;
    setUserData((prevData) => ({
      ...prevData,
      [name]: value,
    }));
  };

  useEffect(() => {
    fetchProfile();
  }, []);

  const handleSave = async () => {
    try {
      console.log("userData:", userData);
      await api.put(
        `${DCAPAL_API}/v1/user/investment-preferences`,
        userData,
        config
      );
      setIsEditing(false);
      fetchProfile(); // Refresh the data after updating
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
          <div className="w-full max-w-4xl p-4 bg-white rounded-lg shadow-md">
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
                    onClick={() => handleMenuItemClick("/investment-settings")}
                  >
                    Investment Settings
                  </MenuItem>
                </MenuList>
              </Menu>
            </div>
            <div className="p-4 space-y-6">
              <div className="flex items-center space-x-4">
                <label className="w-1/4 text-lg font-semibold">
                  Investment Mode
                </label>
                {isEditing ? (
                  <Select
                    name="investment_mode"
                    value={userData.investment_mode}
                    className="w-3/4"
                    isReadOnly={!isEditing}
                    onChange={handleInputChange}
                  >
                    <option value="Standard">Standard</option>
                    <option value="Expert">Expert</option>
                  </Select>
                ) : (
                  <Input
                    value={userData.investment_mode}
                    className="w-3/4"
                    readOnly={true}
                  />
                )}
              </div>
              <div className="flex items-center space-x-4">
                <label className="w-1/4 text-lg font-semibold">
                  Time Horizon
                </label>
                {isEditing ? (
                  <NumberInput
                    name="investment_horizon"
                    value={userData.investment_horizon}
                    min={10}
                    max={20}
                    className="w-full"
                    isReadOnly={!isEditing}
                    onChange={handleInputChange}
                  >
                    <NumberInputField />
                    <NumberInputStepper>
                      <NumberIncrementStepper />
                      <NumberDecrementStepper />
                    </NumberInputStepper>
                  </NumberInput>
                ) : (
                  <Input
                    value={userData.investment_horizon}
                    className="w-3/4"
                    readOnly={true}
                  />
                )}
              </div>
              <div className="flex items-center space-x-4">
                <label className="w-1/4 text-lg font-semibold">
                  Risk Tolerance
                </label>
                {isEditing ? (
                  <Select
                    name="risk_tolerance"
                    value={userData.risk_tolerance}
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
                    value={userData.risk_tolerance}
                    className="w-3/4"
                    readOnly={true}
                  />
                )}
              </div>
              <div className="flex items-center space-x-4">
                <label className="w-1/4 text-lg font-semibold">
                  Investment Goal
                </label>
                {isEditing ? (
                  <Select
                    name="investment_goal"
                    value={userData.investment_goal}
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
                    value={userData.investment_goal}
                    className="w-3/4"
                    readOnly={true}
                  />
                )}
              </div>
              <div className="flex items-center space-x-4">
                <label className="w-1/4 text-lg font-semibold">AI*</label>
                <FormControl display="flex" alignItems="center">
                  <FormLabel htmlFor="email-alerts" mb="0">
                    Enable
                  </FormLabel>
                  <Switch
                    id="email-alerts"
                    isReadOnly={!isEditing}
                    isChecked={userData.ai_enabled}
                    onChange={handleInputChange}
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
              *Requires Personal Data Usage
            </Text>
          </div>
        </div>
      }
    />
  );
}
