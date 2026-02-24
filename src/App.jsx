import { useState, useCallback } from "react";
import "./App.css";
import ServerList from "./components/ServerList";
import icon from "./assets/icon-128x128.png";

function App() {
  const [serverCount, setServerCount] = useState(0);
  const [scanFn, setScanFn] = useState(null);

  const onServersChange = useCallback((count, scan) => {
    setServerCount(count);
    setScanFn(() => scan);
  }, []);

  return (
    <div className="container">
      <div className="main-panel">
        <main>
          <ServerList onServersChange={onServersChange} />
        </main>
        <header>
          <div className="header-brand">
            <img src={icon} alt="Home127 Icon" className="app-icon" />
            <h1>Home127</h1>
          </div>
          <div className="header-actions">
            <span className="server-count">{serverCount} servers</span>
            <button
              className="header-refresh-btn"
              onClick={() => scanFn?.()}
              title="Refresh"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2.5"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <polyline points="23 4 23 10 17 10"></polyline>
                <polyline points="1 20 1 14 7 14"></polyline>
                <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path>
              </svg>
            </button>
          </div>
        </header>
      </div>
    </div>
  );
}

export default App;
