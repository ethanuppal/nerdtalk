import { useEffect, useState } from "react";
import TextBox from "@components/TextBox";

interface Message {
  body: string;
  timestamp: string;
  sender: string;
  slotnum?: number;
}

export default function MessageLog() {
  const [log, setLog] = useState<Message[]>([]);

  // FIXME: Just for testing purposes
  useEffect(() => {
    const msg1: Message = { body: "test1", timestamp: "hi", sender: "Haadi" };
    const msg2: Message = { body: "test2", timestamp: "hi2", sender: "Ethan" };
    setLog([msg1, msg2]);
  }, []);

  return (
    <div className="flex-grow bg-secondary-color border rounded flex flex-col gap-1 p-1">
      {log.map((message) => (
        <TextBox timestamp={message.timestamp} messageBody={message.body} />
      ))}
    </div>
  );
}
