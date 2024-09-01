import React, { useEffect, useRef, useState } from "react";
import { api } from "@app/api";
import { DCAPAL_API } from "@app/config";
import { Button } from "@chakra-ui/react";

export function ChatCard(props) {
  const [messages, setMessages] = useState([]);
  const messagesEndRef = useRef(null);
  const [chatHeight, setChatHeight] = useState(0);
  const chatContainerRef = useRef(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  const handleSendMessage = async (message) => {
    if (message.trim() === "") return;

    const newMessage = { text: message, sender: "user" };
    setMessages((prevMessages) => [...prevMessages, newMessage]);

    try {
      const response = await api.post(
        `${DCAPAL_API}/v1/ai/chatbot`,
        { message: message, portfolio: props.portfolioId },
        props.config
      );
      const aiResponse = { text: response.data.message, sender: "ai" };
      setMessages((prevMessages) => [...prevMessages, aiResponse]);
    } catch (error) {
      console.error("Error sending message:", error);
    }
  };

  useEffect(() => {
    const fetchInitialMessage = async () => {
      try {
        const response = await api.post(
          `${DCAPAL_API}/v1/ai/chatbot`,
          { message: "initial", portfolio: props.portfolioId },
          props.config
        );
        const aiResponse = { text: response.data.message, sender: "ai" };
        setMessages([aiResponse]);
      } catch (error) {
        console.error("Error fetching initial message:", error);
        setMessages([
          {
            text: "Failed to load initial message. Please try again later.",
            sender: "ai",
          },
        ]);
      }
    };

    fetchInitialMessage();
  }, []);

  useEffect(scrollToBottom, [messages]);

  useEffect(() => {
    const updateHeight = () => {
      if (chatContainerRef.current) {
        const containerHeight = chatContainerRef.current.offsetHeight;
        setChatHeight(containerHeight - 120); // Subtracting space for buttons
      }
    };

    updateHeight();
    window.addEventListener("resize", updateHeight);
    return () => window.removeEventListener("resize", updateHeight);
  }, []);

  return (
    <div
      ref={chatContainerRef}
      className="flex flex-col bg-white rounded-lg shadow-lg h-full"
    >
      <div
        className="flex-1 overflow-y-scroll p-4 scrollbar-thin scrollbar-thumb-gray-300 scrollbar-track-gray-100"
        style={{ height: `${chatHeight}px`, maxHeight: `${chatHeight}px` }}
      >
        {messages.map((msg, index) => (
          <div
            key={index}
            className={`mb-4 ${
              msg.sender === "user" ? "text-right" : "text-left"
            }`}
          >
            <span
              className={`inline-block p-3 rounded-lg break-words max-w-[80%] ${
                msg.sender === "user" ? "bg-blue-100" : "bg-gray-100"
              }`}
            >
              {msg.text}
            </span>
          </div>
        ))}
        <div ref={messagesEndRef} />
      </div>
      <div className="p-4 border-t">
        <div className="flex flex-col space-y-2">
          <Button
            onClick={() => handleSendMessage("market-news")}
            className="w-full"
          >
            Market News
          </Button>
          <Button
            onClick={() => handleSendMessage("rebalance-portfolio")}
            className="w-full"
          >
            Rebalance Portfolio
          </Button>
        </div>
      </div>
    </div>
  );
}
