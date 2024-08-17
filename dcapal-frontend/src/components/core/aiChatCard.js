import React, { useEffect, useState } from "react";
import { api } from "@app/api";
import { DCAPAL_API } from "@app/config";

export function ChatCard(props) {
  const [input, setInput] = useState("help");
  const [messages, setMessages] = useState([]);

  const handleSendMessage = async () => {
    if (input.trim() === "") return;

    const newMessage = { text: input, sender: "user" };
    // setMessages([...messages, newMessage]);
    setInput("");

    try {
      const response = await api.post(
        `${DCAPAL_API}/v1/ai/chatbot`,
        { message: input },
        props.config
      );
      const aiResponse = { text: response.data.message, sender: "ai" };
      setMessages((prevMessages) => [...prevMessages, aiResponse]);
    } catch (error) {
      console.error("Error sending message:", error);
    }
  };

  useEffect(() => {
    const aiResponse = {
      text: "Based on the data provided, it appears that you have a diversified investment portfolio with allocations to stocks, bonds, and cryptocurrencies. It's important to note that investment decisions should be based on your individual financial goals, risk tolerance, and time horizon. \nConsidering your age, income, and the current allocation of your investments, adding more investments into stocks, bonds, or other diversified assets might be more suitable to ensure a well-rounded portfolio. However, it's essential to consult with a financial advisor to assess your specific financial situation and investment objectives.\nPlease remember that this response is not financial advice, but rather a general assessment based on the information provided.",
      sender: "ai",
    };

    setMessages([aiResponse]);
  }, []); // Empty dependency array means this effect runs once on mount

  /*
                                                          if (props.isChatVisible) {
                                                            handleSendMessage();
                                                            }
                                                           */

  return (
    <div className="flex-1 overflow-y-auto bg-white rounded-lg shadow-lg">
      {messages.map((msg, index) => (
        <div
          key={index}
          className={`mb-2 ${msg.sender === "user" ? "text-right" : "text-left"}`}
        >
          <span
            className={`mx-2 inline-block p-2 rounded-lg ${msg.sender === "user" ? "bg-blue-100" : "bg-white"}`}
          >
            {msg.text}
          </span>
        </div>
      ))}
      {/*      <div className="mt-auto">
        <div className="flex">
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            className="flex-1 p-2 border rounded-l-lg"
            placeholder="Type your message..."
          />
          <Button onClick={handleSendMessage} className="rounded-r-lg">
            Send
          </Button>
        </div>
      </div>*/}
    </div>
  );
}
