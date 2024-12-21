// import "./App.css";

import InputBox from "@components/InputBox";
import MessageLogDisplay from "@components/MessageLogDisplay";
import messageLogStore from "@state/messageLogStore";

function App() {
  return (
    <main className="h-screen flex flex-col gap-2 p-2 border-none">
      <MessageLogDisplay messageLogStore={messageLogStore} />

      <InputBox messageLogStore={messageLogStore} />
    </main>
  );
}

export default App;
