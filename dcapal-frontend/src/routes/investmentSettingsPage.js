import React, { useEffect, useState } from "react";
import { DCAPAL_API } from "@app/config";
import { api } from "@app/api";
import { ContainerPage } from "./containerPage";
import { useNavigate } from "react-router-dom";
import {
  Button,
  FormControl,
  FormLabel,
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
  const [profileData, setProfileData] = useState(null);
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
      const response = await api.get(`${DCAPAL_API}/v1/user/profile`, config);
      setProfileData(response.data);
      console.log("Profile data:", response.data);
    } catch (error) {
      console.error("Error fetching data:", error);
    }
  };

  useEffect(() => {
    fetchProfile();
  }, []);

  const handleSave = async () => {
    try {
      await api.put(`${DCAPAL_API}/v1/user/profile`, userData);
      setIsEditing(false);
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
                <Select
                  value={profileData?.first_name + " " + profileData?.last_name}
                  className="w-3/4"
                  isReadOnly={!isEditing}
                >
                  <option value="option1">Option 1</option>
                  <option value="option2">Option 2</option>
                  <option value="option3">Option 3</option>
                </Select>
              </div>
              <div className="flex items-center space-x-4">
                <label className="w-1/4 text-lg font-semibold">
                  Time Horizon
                </label>
                <NumberInput
                  defaultValue={profileData?.birth_date}
                  min={10}
                  max={20}
                  className="w-full"
                  isReadOnly={!isEditing}
                >
                  <NumberInputField />
                  <NumberInputStepper>
                    <NumberIncrementStepper />
                    <NumberDecrementStepper />
                  </NumberInputStepper>
                </NumberInput>
              </div>
              <div className="flex items-center space-x-4">
                <label className="w-1/4 text-lg font-semibold">
                  Risk Tolerance
                </label>
                <Select
                  value={profileData?.first_name + " " + profileData?.last_name}
                  className="w-3/4"
                  isReadOnly={!isEditing}
                >
                  <option value="option1">Option 1</option>
                  <option value="option2">Option 2</option>
                  <option value="option3">Option 3</option>
                </Select>
              </div>
              <div className="flex items-center space-x-4">
                <label className="w-1/4 text-lg font-semibold">
                  Investment Goal
                </label>
                <Select
                  value={profileData?.first_name + " " + profileData?.last_name}
                  className="w-3/4"
                  isReadOnly={!isEditing}
                >
                  <option value="option1">Option 1</option>
                  <option value="option2">Option 2</option>
                  <option value="option3">Option 3</option>
                </Select>
              </div>
              <div className="flex items-center space-x-4">
                <label className="w-1/4 text-lg font-semibold">AI*</label>
                <FormControl display="flex" alignItems="center">
                  <FormLabel htmlFor="email-alerts" mb="0">
                    Enable
                  </FormLabel>
                  <Switch id="email-alerts" isReadOnly={!isEditing} />
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
