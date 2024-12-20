// import "./App.css";

import InputBox from "@components/InputBox";
import MessageLog from "@components/MessageLog";

function App() {
  return (
    <main className="h-screen flex flex-col gap-2 p-2">
      <MessageLog />

      <InputBox />
    </main>
  );
}

export default App;
