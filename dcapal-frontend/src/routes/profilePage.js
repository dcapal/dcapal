import React, { useEffect, useState } from "react";
import { DCAPAL_API, supabase } from "@app/config";
import { api } from "@app/api";
import { Button, Input } from "@chakra-ui/react";
import { ContainerPage } from "./containerPage";

export default function Account({ session }) {
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
      title={"Profile"}
      content={
        // flex flex-col items-center justify-center min-h-screen bg-gray-100
        <div className="w-full flex flex-col grow justify-center items-center text-center gap-8 bg-gray-100">
          <div className="w-full max-w-4xl p-4 bg-white rounded-lg shadow-md">
            <div className="flex items-center justify-between p-4 border-b">
              <h2 className="text-lg font-semibold">Personal Data</h2>
            </div>
            <div className="p-4 space-y-6">
              <div className="flex items-center space-x-4">
                <label className="w-1/4 text-lg font-semibold">Full name</label>
                <Input
                  placeholder={session.user.user_metadata.full_name}
                  className="w-3/4"
                />
              </div>
              <div className="flex items-center space-x-4">
                <label className="w-1/4 text-lg font-semibold">
                  Birth Date
                </label>
                <Input
                  placeholder="Select Date and Time"
                  size="md"
                  type="date"
                />
              </div>
              <div className="flex items-center space-x-4">
                <label className="w-1/4 text-lg font-semibold">Email</label>
                <Input
                  placeholder={session.user.email}
                  className="w-3/4"
                  type="email"
                />
              </div>
            </div>
            <div className="flex justify-end p-4 border-t">
              <Button>Confirm</Button>
            </div>
          </div>
        </div>
      }
    />
  );
}
