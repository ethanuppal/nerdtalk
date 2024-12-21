import { MessageLogStore } from "@state/messageLogStore";
import { observer } from "mobx-react-lite";
import { FormEvent, useState } from "react";

interface InputBoxProps {
  messageLogStore: MessageLogStore;
}

const InputBox = observer(({messageLogStore}: InputBoxProps) => {
  const [userInput, setUserInput] = useState("");

  const handleOnSubmit = (e: FormEvent) => {
    e.preventDefault();

    messageLogStore.appendMessage({body: userInput, sender: "HOST", timestamp: "default"})

    setUserInput("");
  };

  return (
    <form
      className="bg-primary-color flex gap-2 rounded border px-2 py-2"
      onSubmit={handleOnSubmit}
    >
      <button>&gt;</button>
      <input
        className="bg-transparent focus:outline-none w-full"
        type="text"
        placeholder="Enter message..."
        value={userInput}
        onChange={(e) => setUserInput(e.target.value)}
      />
    </form>
  );
});

export default InputBox;