import React, { useEffect, useState } from "react";
import { DCAPAL_API } from "@app/config";
import { api } from "@app/api";
import { ContainerPage } from "./containerPage";
import { useNavigate } from "react-router-dom";
import { ChevronDownIcon } from "@chakra-ui/icons";
import {
  Button,
  Input,
  Menu,
  MenuButton,
  MenuItem,
  MenuList,
  useToast,
} from "@chakra-ui/react";

export default function Account({ session }) {
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
      title={"Profile"}
      content={
        <div className="w-full flex flex-col grow justify-center items-center gap-8 bg-gray-100">
          <div className="w-full max-w-4xl p-4 bg-white rounded-lg shadow-md">
            <div className="flex items-center justify-between p-4 border-b">
              <Menu>
                <MenuButton
                  as="div"
                  className="flex items-center cursor-pointer large-text"
                  fontWeight="semibold"
                >
                  Personal Data
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
                <label className="w-1/4 text-lg font-semibold">Full name</label>
                <Input
                  value={profileData?.first_name + " " + profileData?.last_name}
                  className="w-3/4"
                  isReadOnly={!isEditing}
                />
              </div>
              <div className="flex items-center space-x-4">
                <label className="w-1/4 text-lg font-semibold">
                  Birth Date
                </label>
                <Input
                  value={profileData?.birth_date}
                  className="w-3/4"
                  isReadOnly={!isEditing}
                  size="md"
                  type="date"
                />
              </div>
              <div className="flex items-center space-x-4">
                <label className="w-1/4 text-lg font-semibold">Email</label>
                <Input
                  value={profileData?.email}
                  className="w-3/4"
                  isReadOnly={!isEditing}
                  type="email"
                />
              </div>
            </div>
            <div className="flex justify-end p-4 border-t">
              {isEditing ? (
                <Button onClick={handleSave}>Confirm</Button>
              ) : (
                <Button onClick={() => setIsEditing(true)}>Edit</Button>
              )}
            </div>
          </div>
        </div>
      }
    />
  );
}
