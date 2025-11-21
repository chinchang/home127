import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

function ServerList() {
  const [servers, setServers] = useState([]);
  const [loading, setLoading] = useState(true);

  const scan = async () => {
    setLoading(true);
    try {
      const result = await invoke("scan_servers");
      setServers(result);
    } catch (error) {
      console.error("Failed to scan servers:", error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    scan();
    // Optional: Poll every 5 seconds
    const interval = setInterval(scan, 5000);
    return () => clearInterval(interval);
  }, []);

  const handleVisit = async (url) => {
    try {
      await openUrl(url);
    } catch (error) {
      console.error("Failed to open URL:", error);
    }
  };

  if (loading && servers.length === 0) {
    return <div className="loading">Scanning...</div>;
  }

  if (servers.length === 0) {
    return (
      <div className="empty-state">
        <p>No servers detected.</p>
        <button onClick={scan} className="refresh-btn">
          Refresh
        </button>
      </div>
    );
  }

  return (
    <div className="server-list">
      {servers.map((server) => (
        <div key={server.port} className="server-item">
          <div className="server-info">
            <span className="server-port">:{server.port}</span>
            <span className="server-title" title={server.title}>
              {server.title}
            </span>
          </div>
          <button
            className="visit-btn"
            onClick={() => handleVisit(server.url)}
            title="Open in Browser"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
              <polyline points="15 3 21 3 21 9"></polyline>
              <line x1="10" y1="14" x2="21" y2="3"></line>
            </svg>
          </button>
        </div>
      ))}
    </div>
  );
}

export default ServerList;
